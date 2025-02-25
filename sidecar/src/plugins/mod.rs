use axum::{
    routing::{get, post},
    Router,
    Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginInfo {
    name: String,
    version: String,
    status: PluginStatus,
    config: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginStatus {
    Active,
    Inactive,
    Error,
}

#[derive(Debug, Deserialize)]
pub struct InstallRequest {
    name: String,
    version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigureRequest {
    name: String,
    config: HashMap<String, serde_json::Value>,
}

pub fn router() -> Router {
    Router::new()
        .route("/plugins/list", get(list_plugins))
        .route("/plugins/install", post(install_plugin))
        .route("/plugins/configure", post(configure_plugin))
        .route("/plugins/status", get(plugin_status))
}

async fn list_plugins() -> Json<Vec<String>> {
    Json(vec![
        "git-integration".to_string(),
        "code-formatter".to_string(),
        "language-detector".to_string(),
    ])
}

async fn install_plugin(Json(req): Json<InstallRequest>) -> Result<StatusCode, StatusCode> {
    // In a real implementation, this would install the plugin
    Ok(StatusCode::OK)
}

async fn configure_plugin(
    Json(req): Json<ConfigureRequest>,
) -> Result<StatusCode, StatusCode> {
    // In a real implementation, this would configure the plugin
    Ok(StatusCode::OK)
}

async fn plugin_status() -> Json<HashMap<String, PluginInfo>> {
    let mut status = HashMap::new();
    let mut config = HashMap::new();
    config.insert("enabled".to_string(), serde_json::Value::Bool(true));
    
    status.insert(
        "git-integration".to_string(),
        PluginInfo {
            name: "git-integration".to_string(),
            version: "1.0.0".to_string(),
            status: PluginStatus::Active,
            config,
        },
    );

    Json(status)
}