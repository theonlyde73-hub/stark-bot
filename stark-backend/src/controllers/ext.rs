//! Public `/ext` endpoint — proxies HTTP requests to module external endpoints.
//!
//! Modules can declare `[[ext_endpoints]]` in their `module.toml` to expose
//! public-facing HTTP endpoints. This controller routes incoming requests to
//! the correct module service, acting as a transparent proxy (preserving status
//! codes, headers, and body — critical for x402 payment flows).

use actix_web::{web, HttpRequest, HttpResponse};
use base64::Engine;
use serde::Serialize;
use std::collections::HashMap;

use crate::AppState;

/// Headers to forward from client to module service.
const FORWARD_REQUEST_HEADERS: &[&str] = &[
    "content-type",
    "authorization",
    "x-payment",
    "x-forwarded-for",
];

/// Headers to forward from module service response back to client.
const FORWARD_RESPONSE_HEADERS: &[&str] = &[
    "content-type",
    "payment-required",
    "x-transaction-hash",
    "x-payment-transaction",
];

/// Whether a header name should be forwarded (exact match or x-* prefix).
fn should_forward_request_header(name: &str) -> bool {
    let lower = name.to_lowercase();
    FORWARD_REQUEST_HEADERS.contains(&lower.as_str()) || lower.starts_with("x-")
}

fn should_forward_response_header(name: &str) -> bool {
    let lower = name.to_lowercase();
    FORWARD_RESPONSE_HEADERS.contains(&lower.as_str()) || lower.starts_with("x-")
}

#[derive(Serialize)]
struct ExtDiscoveryEndpoint {
    module_name: String,
    method_name: String,
    description: Option<String>,
    http_methods: Vec<String>,
    url: String,
}

/// GET /ext — discovery endpoint listing all available ext endpoints.
async fn ext_discovery(
    data: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let registry = crate::modules::ModuleRegistry::new();
    let installed = data.db.list_installed_modules().unwrap_or_default();

    let mut endpoints = Vec::new();

    for entry in &installed {
        if !entry.enabled {
            continue;
        }
        if let Some(module) = registry.get(&entry.module_name) {
            for ep in module.ext_endpoint_list() {
                let base = req.connection_info().scheme().to_string();
                let host = req.connection_info().host().to_string();
                endpoints.push(ExtDiscoveryEndpoint {
                    module_name: entry.module_name.clone(),
                    method_name: ep.method_name.clone(),
                    description: ep.description.clone(),
                    http_methods: ep.http_methods.clone(),
                    url: format!("{}://{}/ext/{}/{}", base, host, entry.module_name, ep.method_name),
                });
            }
        }
    }

    HttpResponse::Ok().json(serde_json::json!({
        "endpoints": endpoints,
        "count": endpoints.len(),
    }))
}

/// {any} /ext/{module_name}/{method:.*} — transparent proxy to module ext endpoint.
async fn ext_proxy(
    data: web::Data<AppState>,
    path: web::Path<(String, String)>,
    req: HttpRequest,
    body: web::Bytes,
) -> HttpResponse {
    let (module_name, method) = path.into_inner();

    // Look up module in registry
    let registry = crate::modules::ModuleRegistry::new();
    let module = match registry.get(&module_name) {
        Some(m) => m,
        None => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": format!("Unknown module: '{}'", module_name)
            }));
        }
    };

    // Verify module is installed and enabled
    let installed = data.db.list_installed_modules().unwrap_or_default();
    let module_entry = installed.iter().find(|m| m.module_name == module_name);
    match module_entry {
        None => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": format!("Module '{}' is not installed", module_name)
            }));
        }
        Some(entry) if !entry.enabled => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Module '{}' is disabled", module_name)
            }));
        }
        _ => {}
    }

    // Find the ext endpoint declaration
    let ext_ep = match module.find_ext_endpoint(&method) {
        Some(ep) => ep,
        None => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": format!("No ext endpoint '{}' on module '{}'", method, module_name)
            }));
        }
    };

    // Validate HTTP method
    let request_method = req.method().as_str().to_uppercase();
    let allowed_methods: Vec<String> = ext_ep.http_methods.iter().map(|m| m.to_uppercase()).collect();
    if !allowed_methods.contains(&request_method) {
        return HttpResponse::MethodNotAllowed().json(serde_json::json!({
            "error": format!(
                "Method {} not allowed for /ext/{}/{}. Allowed: {:?}",
                request_method, module_name, method, allowed_methods
            )
        }));
    }

    // Collect headers to forward
    let mut forward_headers = HashMap::new();
    for (key, value) in req.headers() {
        let key_str = key.as_str();
        if should_forward_request_header(key_str) {
            if let Ok(v) = value.to_str() {
                forward_headers.insert(key_str.to_string(), v.to_string());
            }
        }
    }

    // x402 auto-verification: if the endpoint declares x402 = true and the request
    // has an X-Payment header, verify it before forwarding.
    if ext_ep.x402 {
        if let Some(payment_header) = forward_headers.get("x-payment") {
            // Build verification requirements from the endpoint's manifest fields
            let price = ext_ep.x402_price.as_deref().unwrap_or("0");
            let currency = ext_ep.x402_currency.as_deref().unwrap_or("USDC");
            let network = ext_ep.x402_network.as_deref().unwrap_or("base");
            let payee = ext_ep.x402_payee.as_deref().unwrap_or("");

            if !payee.is_empty() {
                match crate::x402::verify::decode_payment_header(payment_header) {
                    Ok(payload_json) => {
                        let requirements = crate::x402::verify::VerifyRequirements {
                            price: price.to_string(),
                            currency: currency.to_string(),
                            payee: payee.to_string(),
                            network: network.to_string(),
                            asset: None,
                            token_name: None,
                            token_version: None,
                            decimals: None,
                        };
                        let result = crate::x402::verify::verify_payment(&payload_json, &requirements);
                        if result.valid {
                            forward_headers.insert(
                                "x-payment-verified".to_string(),
                                "true".to_string(),
                            );
                            forward_headers.insert(
                                "x-payment-payer".to_string(),
                                result.payer,
                            );
                            log::info!(
                                "[EXT] x402 payment verified for /ext/{}/{} — payer: {}",
                                module_name, method,
                                forward_headers.get("x-payment-payer").unwrap_or(&String::new()),
                            );
                        } else {
                            log::warn!(
                                "[EXT] x402 payment verification failed for /ext/{}/{}: {}",
                                module_name, method,
                                result.error.as_deref().unwrap_or("unknown"),
                            );
                            // Don't block — forward without verified header so module can
                            // decide how to handle it. The module sees no X-Payment-Verified.
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "[EXT] Failed to decode X-Payment for /ext/{}/{}: {}",
                            module_name, method, e,
                        );
                    }
                }
            }
        } else {
            // No X-Payment header but endpoint requires x402 — generate a 402 response
            let price = ext_ep.x402_price.as_deref().unwrap_or("0");
            let currency = ext_ep.x402_currency.as_deref().unwrap_or("USDC");
            let network = ext_ep.x402_network.as_deref().unwrap_or("base");
            let payee = ext_ep.x402_payee.as_deref().unwrap_or("");

            if !payee.is_empty() {
                let asset = crate::x402::USDC_ADDRESS;
                let max_amount = crate::x402::verify::parse_token_amount(price, 6)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|_| price.to_string());

                let requirement = serde_json::json!({
                    "scheme": "exact",
                    "network": format!("eip155:{}", crate::x402::chain_id_for_network(network)),
                    "maxAmountRequired": max_amount,
                    "payToAddress": payee,
                    "asset": asset,
                    "maxTimeoutSeconds": 3600,
                });
                let payment_required = serde_json::json!({
                    "x402Version": 1,
                    "accepts": [requirement],
                });
                let encoded = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    serde_json::to_string(&payment_required).unwrap_or_default(),
                );

                return HttpResponse::PaymentRequired()
                    .insert_header(("Payment-Required", encoded))
                    .json(serde_json::json!({
                        "error": "Payment Required",
                        "payment_required": payment_required,
                    }));
            }
        }
    }

    // Proxy the request
    match module
        .proxy_ext_request(&ext_ep.rpc_endpoint, &request_method, body.to_vec(), forward_headers)
        .await
    {
        Ok(proxy_resp) => {
            let status = actix_web::http::StatusCode::from_u16(proxy_resp.status)
                .unwrap_or(actix_web::http::StatusCode::BAD_GATEWAY);
            let mut response = HttpResponse::build(status);

            // Forward response headers
            for (key, value) in &proxy_resp.headers {
                if should_forward_response_header(key) {
                    if let Ok(header_name) = actix_web::http::header::HeaderName::from_bytes(key.as_bytes()) {
                        if let Ok(header_value) = actix_web::http::header::HeaderValue::from_str(value) {
                            response.insert_header((header_name, header_value));
                        }
                    }
                }
            }

            response.body(proxy_resp.body)
        }
        Err(e) => {
            log::error!("[EXT] Proxy error for /ext/{}/{}: {}", module_name, method, e);
            HttpResponse::BadGateway().json(serde_json::json!({
                "error": format!("Failed to proxy request to module: {}", e)
            }))
        }
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/ext")
            .route("", web::get().to(ext_discovery))
            .route("/{module_name}/{method:.*}", web::to(ext_proxy)),
    );
}
