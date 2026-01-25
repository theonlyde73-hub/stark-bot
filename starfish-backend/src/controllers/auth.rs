use actix_web::{web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Deserialize)]
pub struct LoginRequest {
    secret_key: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Deserialize)]
pub struct LogoutRequest {
    token: String,
}

#[derive(Serialize)]
pub struct LogoutResponse {
    success: bool,
}

#[derive(Serialize)]
pub struct ValidateResponse {
    valid: bool,
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/auth")
            .route("/login", web::post().to(login))
            .route("/logout", web::post().to(logout))
            .route("/validate", web::get().to(validate)),
    );
}

async fn login(state: web::Data<AppState>, body: web::Json<LoginRequest>) -> impl Responder {
    if body.secret_key != state.config.secret_key {
        return HttpResponse::Unauthorized().json(LoginResponse {
            success: false,
            token: None,
            error: Some("Invalid secret key".to_string()),
        });
    }

    match state.db.create_session() {
        Ok(session) => HttpResponse::Ok().json(LoginResponse {
            success: true,
            token: Some(session.token),
            error: None,
        }),
        Err(e) => {
            log::error!("Failed to create session: {}", e);
            HttpResponse::InternalServerError().json(LoginResponse {
                success: false,
                token: None,
                error: Some("Failed to create session".to_string()),
            })
        }
    }
}

async fn logout(state: web::Data<AppState>, body: web::Json<LogoutRequest>) -> impl Responder {
    match state.db.delete_session(&body.token) {
        Ok(_) => HttpResponse::Ok().json(LogoutResponse { success: true }),
        Err(e) => {
            log::error!("Failed to delete session: {}", e);
            HttpResponse::InternalServerError().json(LogoutResponse { success: false })
        }
    }
}

async fn validate(state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.trim_start_matches("Bearer ").to_string());

    let token = match token {
        Some(t) => t,
        None => {
            return HttpResponse::Ok().json(ValidateResponse { valid: false });
        }
    };

    match state.db.validate_session(&token) {
        Ok(Some(_)) => HttpResponse::Ok().json(ValidateResponse { valid: true }),
        Ok(None) => HttpResponse::Ok().json(ValidateResponse { valid: false }),
        Err(e) => {
            log::error!("Failed to validate session: {}", e);
            HttpResponse::Ok().json(ValidateResponse { valid: false })
        }
    }
}
