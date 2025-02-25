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

pub fn router() -> Router<Arc<LspState>> {
    Router::new()
        .route("/lsp/start", post(start_server))
        .route("/lsp/status", get(server_status))
        .route("/lsp/configure", post(configure_server))
        .route("/lsp/capabilities", get(get_capabilities))
}

async fn start_server(
    State(state): State<Arc<LspState>>,
    Json(req): Json<StartRequest>
) -> Result<StatusCode, StatusCode> {
    state.start_server(&req.language, req.config)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}

async fn server_status(
    State(state): State<Arc<LspState>>
) -> Json<HashMap<String, LspServerInfo>> {
    let servers = state.servers.read().await;
    let mut status = HashMap::new();
    
    for (language, instance) in servers.iter() {
        status.insert(
            language.clone(),
            LspServerInfo {
                language: language.clone(),
                status: instance.status.clone(),
                pid: Some(instance.pid),
                capabilities: vec![
                    "completions".to_string(),
                    "diagnostics".to_string(),
                    "formatting".to_string(),
                ],
            },
        );
    }
    
    Json(status)
}

async fn configure_server(
    State(state): State<Arc<LspState>>,
    Json(config): Json<LspConfig>
) -> Result<StatusCode, StatusCode> {
    let mut servers = state.servers.write().await;
    if let Some(instance) = servers.get_mut(&config.root_path) {
        instance.config = config;
        Ok(StatusCode::OK)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn get_capabilities(
    State(state): State<Arc<LspState>>
) -> Json<HashMap<String, Vec<String>>> {
    let capabilities = state.capabilities.read().await;
    Json(capabilities.clone())
}