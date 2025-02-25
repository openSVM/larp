use axum::{
    routing::{get, post},
    Router,
    Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct LspConfig {
    root_path: String,
    initialization_options: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LspServerInfo {
    language: String,
    status: LspStatus,
    pid: Option<u32>,
    capabilities: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LspStatus {
    Running,
    Stopped,
    Failed,
}

#[derive(Debug, Deserialize)]
pub struct StartRequest {
    language: String,
    config: LspConfig,
}

pub fn router() -> Router {
    Router::new()
        .route("/lsp/start", post(start_server))
        .route("/lsp/status", get(server_status))
        .route("/lsp/configure", post(configure_server))
        .route("/lsp/capabilities", get(get_capabilities))
}

async fn start_server(Json(req): Json<StartRequest>) -> Result<StatusCode, StatusCode> {
    // In a real implementation, this would start the language server
    Ok(StatusCode::OK)
}

async fn server_status() -> Json<HashMap<String, LspServerInfo>> {
    let mut status = HashMap::new();
    
    status.insert(
        "rust".to_string(),
        LspServerInfo {
            language: "rust".to_string(),
            status: LspStatus::Running,
            pid: Some(1234),
            capabilities: vec![
                "completions".to_string(),
                "diagnostics".to_string(),
                "formatting".to_string(),
            ],
        },
    );

    Json(status)
}

async fn configure_server(
    Json(config): Json<LspConfig>,
) -> Result<StatusCode, StatusCode> {
    // In a real implementation, this would configure the language server
    Ok(StatusCode::OK)
}

async fn get_capabilities() -> Json<HashMap<String, Vec<String>>> {
    let mut capabilities = HashMap::new();
    capabilities.insert(
        "rust".to_string(),
        vec![
            "completions".to_string(),
            "diagnostics".to_string(),
            "formatting".to_string(),
            "references".to_string(),
            "definition".to_string(),
        ],
    );
    Json(capabilities)
}