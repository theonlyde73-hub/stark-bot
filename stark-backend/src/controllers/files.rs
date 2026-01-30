use actix_web::{web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

use crate::config::workspace_dir;
use crate::AppState;

/// Validate session token from request
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

#[derive(Debug, Serialize)]
struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
    modified: Option<String>,
}

#[derive(Debug, Serialize)]
struct ListFilesResponse {
    success: bool,
    path: String,
    entries: Vec<FileEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListFilesQuery {
    path: Option<String>,
}

/// List files in the workspace directory
async fn list_files(
    data: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<ListFilesQuery>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }

    let workspace = workspace_dir();
    let workspace_path = Path::new(&workspace);

    // Resolve the requested path
    let relative_path = query.path.as_deref().unwrap_or("");
    let full_path = if relative_path.is_empty() {
        workspace_path.to_path_buf()
    } else {
        workspace_path.join(relative_path)
    };

    // Security check: canonicalize and ensure we're within workspace
    let canonical_workspace = match workspace_path.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ListFilesResponse {
                success: false,
                path: relative_path.to_string(),
                entries: vec![],
                error: Some(format!("Workspace not accessible: {}", e)),
            });
        }
    };

    let canonical_path = match full_path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // Path doesn't exist - check if workspace exists first
            if !workspace_path.exists() {
                return HttpResponse::Ok().json(ListFilesResponse {
                    success: true,
                    path: relative_path.to_string(),
                    entries: vec![],
                    error: Some("Workspace directory does not exist yet".to_string()),
                });
            }
            return HttpResponse::NotFound().json(ListFilesResponse {
                success: false,
                path: relative_path.to_string(),
                entries: vec![],
                error: Some("Path not found".to_string()),
            });
        }
    };

    // Ensure path is within workspace
    if !canonical_path.starts_with(&canonical_workspace) {
        return HttpResponse::Forbidden().json(ListFilesResponse {
            success: false,
            path: relative_path.to_string(),
            entries: vec![],
            error: Some("Access denied: path outside workspace".to_string()),
        });
    }

    // Read directory contents
    let mut entries = Vec::new();
    let mut read_dir = match fs::read_dir(&canonical_path).await {
        Ok(rd) => rd,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ListFilesResponse {
                success: false,
                path: relative_path.to_string(),
                entries: vec![],
                error: Some(format!("Failed to read directory: {}", e)),
            });
        }
    };

    while let Ok(Some(entry)) = read_dir.next_entry().await {
        let name = match entry.file_name().to_str() {
            Some(n) => n.to_string(),
            None => continue,
        };

        let metadata = match entry.metadata().await {
            Ok(m) => m,
            Err(_) => continue,
        };

        let entry_path = entry.path();
        let rel_path = entry_path
            .strip_prefix(&canonical_workspace)
            .unwrap_or(&entry_path)
            .to_string_lossy()
            .to_string();

        let modified = metadata.modified().ok().map(|t| {
            let datetime: chrono::DateTime<chrono::Utc> = t.into();
            datetime.format("%Y-%m-%d %H:%M:%S").to_string()
        });

        entries.push(FileEntry {
            name,
            path: rel_path,
            is_dir: metadata.is_dir(),
            size: if metadata.is_dir() { 0 } else { metadata.len() },
            modified,
        });
    }

    // Sort: directories first, then by name
    entries.sort_by(|a, b| {
        if a.is_dir != b.is_dir {
            b.is_dir.cmp(&a.is_dir)
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });

    HttpResponse::Ok().json(ListFilesResponse {
        success: true,
        path: relative_path.to_string(),
        entries,
        error: None,
    })
}

#[derive(Debug, Serialize)]
struct ReadFileResponse {
    success: bool,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_binary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReadFileQuery {
    path: String,
}

/// Read a file from the workspace
async fn read_file(
    data: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<ReadFileQuery>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }

    let workspace = workspace_dir();
    let workspace_path = Path::new(&workspace);
    let full_path = workspace_path.join(&query.path);

    // Security check
    let canonical_workspace = match workspace_path.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ReadFileResponse {
                success: false,
                path: query.path.clone(),
                content: None,
                size: None,
                is_binary: None,
                error: Some(format!("Workspace not accessible: {}", e)),
            });
        }
    };

    let canonical_path = match full_path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            return HttpResponse::NotFound().json(ReadFileResponse {
                success: false,
                path: query.path.clone(),
                content: None,
                size: None,
                is_binary: None,
                error: Some("File not found".to_string()),
            });
        }
    };

    if !canonical_path.starts_with(&canonical_workspace) {
        return HttpResponse::Forbidden().json(ReadFileResponse {
            success: false,
            path: query.path.clone(),
            content: None,
            size: None,
            is_binary: None,
            error: Some("Access denied: path outside workspace".to_string()),
        });
    }

    // Check if it's a file
    let metadata = match fs::metadata(&canonical_path).await {
        Ok(m) => m,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ReadFileResponse {
                success: false,
                path: query.path.clone(),
                content: None,
                size: None,
                is_binary: None,
                error: Some(format!("Failed to read file metadata: {}", e)),
            });
        }
    };

    if metadata.is_dir() {
        return HttpResponse::BadRequest().json(ReadFileResponse {
            success: false,
            path: query.path.clone(),
            content: None,
            size: None,
            is_binary: None,
            error: Some("Path is a directory, not a file".to_string()),
        });
    }

    // Read file content (limit to 1MB for safety)
    const MAX_SIZE: u64 = 1024 * 1024;
    if metadata.len() > MAX_SIZE {
        return HttpResponse::Ok().json(ReadFileResponse {
            success: true,
            path: query.path.clone(),
            content: None,
            size: Some(metadata.len()),
            is_binary: Some(true),
            error: Some(format!("File too large to display ({} bytes)", metadata.len())),
        });
    }

    let content = match fs::read(&canonical_path).await {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ReadFileResponse {
                success: false,
                path: query.path.clone(),
                content: None,
                size: None,
                is_binary: None,
                error: Some(format!("Failed to read file: {}", e)),
            });
        }
    };

    // Check if binary
    let is_binary = content.iter().take(8000).any(|&b| b == 0);
    if is_binary {
        return HttpResponse::Ok().json(ReadFileResponse {
            success: true,
            path: query.path.clone(),
            content: None,
            size: Some(metadata.len()),
            is_binary: Some(true),
            error: Some("Binary file cannot be displayed".to_string()),
        });
    }

    let text = String::from_utf8_lossy(&content).to_string();

    HttpResponse::Ok().json(ReadFileResponse {
        success: true,
        path: query.path.clone(),
        content: Some(text),
        size: Some(metadata.len()),
        is_binary: Some(false),
        error: None,
    })
}

#[derive(Debug, Serialize)]
struct WorkspaceInfoResponse {
    success: bool,
    workspace_path: String,
    exists: bool,
}

/// Get workspace info
async fn workspace_info(
    data: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }

    let workspace = workspace_dir();
    let exists = Path::new(&workspace).exists();

    HttpResponse::Ok().json(WorkspaceInfoResponse {
        success: true,
        workspace_path: workspace,
        exists,
    })
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/files")
            .route("", web::get().to(list_files))
            .route("/read", web::get().to(read_file))
            .route("/workspace", web::get().to(workspace_info)),
    );
}
