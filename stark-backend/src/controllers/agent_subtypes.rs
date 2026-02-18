use actix_web::{web, HttpRequest, HttpResponse, Responder};
use serde::Deserialize;

use crate::ai::multi_agent::types::{self, AgentSubtypeConfig};
use crate::AppState;

const MAX_SUBTYPES: usize = 10;

fn validate_session_from_request(
    state: &web::Data<AppState>,
    req: &HttpRequest,
) -> Result<(), HttpResponse> {
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.trim_start_matches("Bearer ").to_string());

    let token = match token {
        Some(t) => t,
        None => {
            return Err(HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "No authorization token provided"
            })));
        }
    };

    match state.db.validate_session(&token) {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Invalid or expired session"
        }))),
        Err(e) => {
            log::error!("Session validation error: {}", e);
            Err(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal server error"
            })))
        }
    }
}

/// Reload the global registry from DB.
fn reload_registry(db: &crate::db::Database) {
    if let Ok(subtypes) = db.list_agent_subtypes() {
        types::load_subtype_registry(subtypes);
    }
}

/// List all agent subtypes.
async fn list_subtypes(
    data: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }

    match data.db.list_agent_subtypes() {
        Ok(subtypes) => HttpResponse::Ok().json(subtypes),
        Err(e) => {
            log::error!("Failed to list agent subtypes: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// Get a single agent subtype by key.
async fn get_subtype(
    data: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }

    let key = path.into_inner();
    match data.db.get_agent_subtype(&key) {
        Ok(Some(subtype)) => HttpResponse::Ok().json(subtype),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": format!("Agent subtype '{}' not found", key)
        })),
        Err(e) => {
            log::error!("Failed to get agent subtype: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

#[derive(Deserialize)]
struct CreateSubtypeRequest {
    key: String,
    label: String,
    emoji: String,
    description: String,
    tool_groups: Vec<String>,
    skill_tags: Vec<String>,
    #[serde(default)]
    additional_tools: Vec<String>,
    prompt: String,
    #[serde(default)]
    sort_order: i32,
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    max_iterations: Option<u32>,
    #[serde(default)]
    skip_task_planner: bool,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    hidden: bool,
}

fn default_true() -> bool {
    true
}

/// Create a new agent subtype.
async fn create_subtype(
    data: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CreateSubtypeRequest>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }

    // Check limit
    match data.db.count_agent_subtypes() {
        Ok(count) if count >= MAX_SUBTYPES as i64 => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Maximum of {} agent subtypes allowed", MAX_SUBTYPES)
            }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }));
        }
        _ => {}
    }

    // Validate key format
    let key = body.key.to_lowercase();
    if key.is_empty() || !key.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Key must be non-empty and contain only alphanumeric characters and underscores"
        }));
    }

    let config = AgentSubtypeConfig {
        key,
        label: body.label.clone(),
        emoji: body.emoji.clone(),
        description: body.description.clone(),
        tool_groups: body.tool_groups.clone(),
        skill_tags: body.skill_tags.clone(),
        additional_tools: body.additional_tools.clone(),
        prompt: body.prompt.clone(),
        sort_order: body.sort_order,
        enabled: body.enabled,
        max_iterations: body.max_iterations.unwrap_or(90),
        skip_task_planner: body.skip_task_planner,
        aliases: body.aliases.clone(),
        hidden: body.hidden,
    };

    match data.db.upsert_agent_subtype(&config) {
        Ok(_) => {
            reload_registry(&data.db);
            HttpResponse::Created().json(config)
        }
        Err(e) => {
            log::error!("Failed to create agent subtype: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

#[derive(Deserialize)]
struct UpdateSubtypeRequest {
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    emoji: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tool_groups: Option<Vec<String>>,
    #[serde(default)]
    skill_tags: Option<Vec<String>>,
    #[serde(default)]
    additional_tools: Option<Vec<String>>,
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    sort_order: Option<i32>,
    #[serde(default)]
    max_iterations: Option<u32>,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    skip_task_planner: Option<bool>,
    #[serde(default)]
    aliases: Option<Vec<String>>,
    #[serde(default)]
    hidden: Option<bool>,
}

/// Update an existing agent subtype.
async fn update_subtype(
    data: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<UpdateSubtypeRequest>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }

    let key = path.into_inner();

    // Get existing
    let existing = match data.db.get_agent_subtype(&key) {
        Ok(Some(s)) => s,
        Ok(None) => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": format!("Agent subtype '{}' not found", key)
            }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }));
        }
    };

    // Merge updates
    let updated = AgentSubtypeConfig {
        key: existing.key,
        label: body.label.clone().unwrap_or(existing.label),
        emoji: body.emoji.clone().unwrap_or(existing.emoji),
        description: body.description.clone().unwrap_or(existing.description),
        tool_groups: body.tool_groups.clone().unwrap_or(existing.tool_groups),
        skill_tags: body.skill_tags.clone().unwrap_or(existing.skill_tags),
        additional_tools: body.additional_tools.clone().unwrap_or(existing.additional_tools),
        prompt: body.prompt.clone().unwrap_or(existing.prompt),
        sort_order: body.sort_order.unwrap_or(existing.sort_order),
        enabled: body.enabled.unwrap_or(existing.enabled),
        max_iterations: body.max_iterations.unwrap_or(existing.max_iterations),
        skip_task_planner: body.skip_task_planner.unwrap_or(existing.skip_task_planner),
        aliases: body.aliases.clone().unwrap_or(existing.aliases),
        hidden: body.hidden.unwrap_or(existing.hidden),
    };

    match data.db.upsert_agent_subtype(&updated) {
        Ok(_) => {
            reload_registry(&data.db);
            HttpResponse::Ok().json(updated)
        }
        Err(e) => {
            log::error!("Failed to update agent subtype: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// Delete an agent subtype.
async fn delete_subtype(
    data: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }

    let key = path.into_inner();

    match data.db.delete_agent_subtype(&key) {
        Ok(true) => {
            reload_registry(&data.db);
            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": format!("Agent subtype '{}' deleted", key)
            }))
        }
        Ok(false) => HttpResponse::NotFound().json(serde_json::json!({
            "error": format!("Agent subtype '{}' not found", key)
        })),
        Err(e) => {
            log::error!("Failed to delete agent subtype: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// Reset agent subtypes to defaults from `defaultagents.ron`.
async fn reset_defaults(
    data: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }

    // Determine config directory
    let config_dir = if std::path::Path::new("./config").exists() {
        std::path::Path::new("./config")
    } else if std::path::Path::new("../config").exists() {
        std::path::Path::new("../config")
    } else {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Config directory not found"
        }));
    };

    // Delete all existing subtypes
    let existing = data.db.list_agent_subtypes().unwrap_or_default();
    for s in &existing {
        let _ = data.db.delete_agent_subtype(&s.key);
    }

    // Load and insert defaults
    let configs = types::load_default_agent_subtypes_from_file(config_dir);
    for config in &configs {
        if let Err(e) = data.db.upsert_agent_subtype(config) {
            log::error!("Failed to insert default subtype '{}': {}", config.key, e);
        }
    }

    reload_registry(&data.db);

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": format!("Reset to {} default agent subtypes", configs.len()),
        "count": configs.len()
    }))
}

/// Export all agent subtypes as RON.
async fn export_subtypes(
    data: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }

    match data.db.list_agent_subtypes() {
        Ok(subtypes) => {
            let pretty = ron::ser::PrettyConfig::new()
                .depth_limit(3)
                .separate_tuple_members(true)
                .enumerate_arrays(false);
            match ron::ser::to_string_pretty(&subtypes, pretty) {
                Ok(ron_str) => HttpResponse::Ok()
                    .content_type("application/ron")
                    .insert_header(("Content-Disposition", "attachment; filename=\"agent_subtypes.ron\""))
                    .body(ron_str),
                Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("Failed to serialize: {}", e)
                })),
            }
        }
        Err(e) => {
            log::error!("Failed to export agent subtypes: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// Export a single agent subtype as RON (wrapped in a vec for import compatibility).
async fn export_single_subtype(
    data: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }

    let key = path.into_inner();
    match data.db.get_agent_subtype(&key) {
        Ok(Some(subtype)) => {
            let pretty = ron::ser::PrettyConfig::new()
                .depth_limit(3)
                .separate_tuple_members(true)
                .enumerate_arrays(false);
            let wrapped = vec![subtype];
            match ron::ser::to_string_pretty(&wrapped, pretty) {
                Ok(ron_str) => HttpResponse::Ok()
                    .content_type("application/ron")
                    .insert_header(("Content-Disposition", format!("attachment; filename=\"{}.ron\"", key)))
                    .body(ron_str),
                Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("Failed to serialize: {}", e)
                })),
            }
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": format!("Agent subtype '{}' not found", key)
        })),
        Err(e) => {
            log::error!("Failed to export agent subtype '{}': {}", key, e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

#[derive(Deserialize)]
struct ImportRequest {
    ron: String,
    /// If true, delete all existing subtypes before importing
    #[serde(default)]
    replace: bool,
}

/// Import agent subtypes from RON.
async fn import_subtypes(
    data: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<ImportRequest>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }

    // Parse RON
    let configs: Vec<AgentSubtypeConfig> = match ron::from_str(&body.ron) {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Invalid RON: {}", e)
            }));
        }
    };

    if configs.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "No subtypes found in import data"
        }));
    }

    // Check limit
    if !body.replace {
        let existing_count = data.db.count_agent_subtypes().unwrap_or(0) as usize;
        if existing_count + configs.len() > MAX_SUBTYPES {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!(
                    "Import would exceed maximum of {} subtypes ({} existing + {} imported)",
                    MAX_SUBTYPES, existing_count, configs.len()
                )
            }));
        }
    }

    // If replace mode, delete all existing first
    if body.replace {
        let existing = data.db.list_agent_subtypes().unwrap_or_default();
        for s in &existing {
            let _ = data.db.delete_agent_subtype(&s.key);
        }
    }

    // Insert imported subtypes
    let mut imported = 0;
    let mut errors = Vec::new();
    for config in &configs {
        // Validate key
        if config.key.is_empty() || !config.key.chars().all(|c| c.is_alphanumeric() || c == '_') {
            errors.push(format!("Invalid key: '{}'", config.key));
            continue;
        }
        match data.db.upsert_agent_subtype(config) {
            Ok(_) => imported += 1,
            Err(e) => errors.push(format!("Failed to import '{}': {}", config.key, e)),
        }
    }

    reload_registry(&data.db);

    let mut response = serde_json::json!({
        "success": true,
        "imported": imported,
        "total": configs.len(),
        "message": format!("Imported {} of {} subtypes", imported, configs.len())
    });

    if !errors.is_empty() {
        response["errors"] = serde_json::json!(errors);
    }

    HttpResponse::Ok().json(response)
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/agent-subtypes")
            .route("", web::get().to(list_subtypes))
            .route("/reset-defaults", web::post().to(reset_defaults))
            .route("/export", web::get().to(export_subtypes))
            .route("/import", web::post().to(import_subtypes))
            .route("/{key}/export", web::get().to(export_single_subtype))
            .route("/{key}", web::get().to(get_subtype))
            .route("", web::post().to(create_subtype))
            .route("/{key}", web::put().to(update_subtype))
            .route("/{key}", web::delete().to(delete_subtype)),
    );
}
