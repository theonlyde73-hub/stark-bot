use actix_web::{web, HttpResponse, Responder};
use tokio::fs;

/// Serve the agent registration file at /.well-known/agent-registration.json
/// This is a PUBLIC endpoint (no auth) per EIP-8004 for domain verification.
async fn agent_registration() -> impl Responder {
    let path = crate::config::identity_document_path();

    match fs::read_to_string(&path).await {
        Ok(content) => {
            // Parse to validate it's valid JSON, then return with correct content type
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(json) => HttpResponse::Ok()
                    .content_type("application/json")
                    .json(json),
                Err(e) => {
                    log::error!("Invalid JSON in identity file: {}", e);
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "Agent registration file contains invalid JSON"
                    }))
                }
            }
        }
        Err(e) => {
            log::warn!("Agent registration file not found at {:?}: {}", path, e);
            HttpResponse::NotFound().json(serde_json::json!({
                "error": "Agent registration file not configured"
            }))
        }
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/.well-known")
            .route("/agent-registration.json", web::get().to(agent_registration)),
    );
}
