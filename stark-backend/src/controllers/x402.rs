//! x402 RPC endpoints for module-side payment verification.
//!
//! Provides two endpoints:
//! - `POST /rpc/x402/verify`           — verify an x402 payment signature
//! - `POST /rpc/x402/payment-required` — generate a 402 response payload

use actix_web::{web, HttpResponse};
use serde::Deserialize;

use crate::x402::verify::{self, VerifyRequirements};

/// Request body for POST /rpc/x402/verify
#[derive(Debug, Deserialize)]
struct VerifyRequest {
    /// Base64-encoded (or raw JSON) X-Payment header value
    payment_header: String,
    /// Requirements the payment must satisfy
    requirements: VerifyRequirements,
}

/// Request body for POST /rpc/x402/payment-required
#[derive(Debug, Deserialize)]
struct PaymentRequiredRequest {
    price: String,
    #[serde(default = "default_currency")]
    currency: String,
    payee: String,
    #[serde(default = "default_network")]
    network: String,
    #[serde(default)]
    description: Option<String>,
    /// Token contract address (optional, defaults to USDC on the network)
    #[serde(default)]
    asset: Option<String>,
    /// Scheme: "exact" or "permit" (defaults to "exact")
    #[serde(default = "default_scheme")]
    scheme: String,
    /// Extra fields to include (e.g. token metadata)
    #[serde(default)]
    extra: Option<serde_json::Value>,
}

fn default_currency() -> String { "USDC".to_string() }
fn default_network() -> String { "base".to_string() }
fn default_scheme() -> String { "exact".to_string() }

/// POST /rpc/x402/verify — verify an x402 payment signature.
async fn verify_payment(body: web::Json<VerifyRequest>) -> HttpResponse {
    // Decode the payment header
    let payload_json = match verify::decode_payment_header(&body.payment_header) {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "valid": false,
                "error": format!("Failed to decode payment header: {}", e),
            }));
        }
    };

    // Verify the payment
    let result = verify::verify_payment(&payload_json, &body.requirements);
    if result.valid {
        HttpResponse::Ok().json(result)
    } else {
        // Return 200 with valid=false (not an HTTP error — the endpoint worked, the payment didn't pass)
        HttpResponse::Ok().json(result)
    }
}

/// POST /rpc/x402/payment-required — generate a properly formatted 402 response payload.
async fn generate_payment_required(body: web::Json<PaymentRequiredRequest>) -> HttpResponse {
    let asset = body.asset.clone().unwrap_or_else(|| {
        crate::x402::USDC_ADDRESS.to_string()
    });

    // Convert human-readable price to smallest unit (assume 6 decimals for USDC)
    let decimals = 6u8;
    let max_amount = match crate::x402::verify::parse_token_amount(&body.price, decimals) {
        Ok(v) => v.to_string(),
        Err(_) => body.price.clone(), // pass through if already raw
    };

    let mut requirement = serde_json::json!({
        "scheme": body.scheme,
        "network": format!("eip155:{}", crate::x402::chain_id_for_network(&body.network)),
        "maxAmountRequired": max_amount,
        "payToAddress": body.payee,
        "asset": asset,
        "maxTimeoutSeconds": 3600,
    });

    if let Some(ref desc) = body.description {
        requirement["description"] = serde_json::json!(desc);
    }
    if let Some(ref extra) = body.extra {
        requirement["extra"] = extra.clone();
    }

    let payment_required = serde_json::json!({
        "x402Version": 1,
        "accepts": [requirement],
    });

    // Base64-encode for the header
    let encoded = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        serde_json::to_string(&payment_required).unwrap_or_default(),
    );

    HttpResponse::Ok().json(serde_json::json!({
        "status": 402,
        "body": {
            "error": "Payment Required",
            "payment_required": payment_required,
        },
        "headers": {
            "Payment-Required": encoded,
            "Content-Type": "application/json",
        },
    }))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/rpc/x402")
            .route("/verify", web::post().to(verify_payment))
            .route("/payment-required", web::post().to(generate_payment_required)),
    );
}
