//! Key/Value store controller — CRUD endpoints for the Redis-backed KV store.

use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Serialize)]
struct KvEntryResponse {
    key: String,
    value: String,
}

#[derive(Deserialize)]
struct UpsertRequest {
    key: String,
    value: String,
}

#[derive(Deserialize)]
struct DeleteRequest {
    key: String,
}

/// GET /api/kv — list all entries
async fn list_kv(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    if let Err(resp) = crate::controllers::validate_session(&state, &req) {
        return resp;
    }

    let kv = match &state.kv_store {
        Some(store) => store,
        None => {
            return HttpResponse::ServiceUnavailable().json(serde_json::json!({
                "error": "KV store not available (Redis not connected)"
            }));
        }
    };

    match kv.dump_all().await {
        Ok(entries) => {
            let items: Vec<KvEntryResponse> = entries
                .into_iter()
                .map(|(key, value)| KvEntryResponse { key, value })
                .collect();
            HttpResponse::Ok().json(items)
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to list KV entries: {}", e)
        })),
    }
}

/// POST /api/kv — upsert a key/value pair
async fn upsert_kv(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<UpsertRequest>,
) -> HttpResponse {
    if let Err(resp) = crate::controllers::validate_session(&state, &req) {
        return resp;
    }

    let kv = match &state.kv_store {
        Some(store) => store,
        None => {
            return HttpResponse::ServiceUnavailable().json(serde_json::json!({
                "error": "KV store not available (Redis not connected)"
            }));
        }
    };

    // Validate key
    let key = body.key.trim().to_ascii_uppercase();
    if key.is_empty() || key.len() > 128 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Key must be 1-128 characters"
        }));
    }
    if !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Key must contain only letters, digits, and underscores"
        }));
    }

    match kv.set(&key, &body.value).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({
            "key": key,
            "value": body.value,
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to set key: {}", e)
        })),
    }
}

/// DELETE /api/kv — delete a key
async fn delete_kv(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<DeleteRequest>,
) -> HttpResponse {
    if let Err(resp) = crate::controllers::validate_session(&state, &req) {
        return resp;
    }

    let kv = match &state.kv_store {
        Some(store) => store,
        None => {
            return HttpResponse::ServiceUnavailable().json(serde_json::json!({
                "error": "KV store not available (Redis not connected)"
            }));
        }
    };

    let key = body.key.trim().to_ascii_uppercase();
    match kv.delete(&key).await {
        Ok(deleted) => HttpResponse::Ok().json(serde_json::json!({
            "key": key,
            "deleted": deleted,
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to delete key: {}", e)
        })),
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/kv")
            .route("", web::get().to(list_kv))
            .route("", web::post().to(upsert_kv))
            .route("", web::delete().to(delete_kv)),
    );
}
