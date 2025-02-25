use axum::{
    routing::{get, post},
    Router,
    Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelConfig {
    temperature: f32,
    max_tokens: u32,
    top_p: f32,
    frequency_penalty: f32,
    presence_penalty: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    name: String,
    status: ModelStatus,
    config: ModelConfig,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelStatus {
    Available,
    Busy,
    Offline,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigureRequest {
    model_name: String,
    config: ModelConfig,
}

pub fn router() -> Router<Arc<ModelState>> {
    Router::new()
        .route("/models/list", get(list_models))
        .route("/models/configure", post(configure_model))
        .route("/models/status", get(check_status))
}

async fn list_models() -> Json<Vec<String>> {
    let models = vec![
        LLMType::Gpt4.to_string(),
        LLMType::GPT3_5_16k.to_string(),
        LLMType::ClaudeOpus.to_string(),
        LLMType::ClaudeSonnet.to_string(),
        LLMType::CodeLLama70BInstruct.to_string(),
        LLMType::MistralInstruct.to_string(),
        LLMType::GeminiPro.to_string(),
    ];
    Json(models)
}

async fn configure_model(
    State(state): State<Arc<ModelState>>,
    Json(req): Json<ConfigureRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut configs = state.configs.write().await;
    configs.insert(req.model_name, req.config);
    Ok(StatusCode::OK)
}

async fn check_status(
    State(state): State<Arc<ModelState>>,
) -> Json<HashMap<String, ModelInfo>> {
    let mut status = HashMap::new();
    let configs = state.configs.read().await;
    let status_cache = state.status_cache.read().await;

    // Check each provider's status
    for provider in state.broker.providers.keys() {
        match provider {
            LLMProvider::OpenAI => {
                add_model_status(&mut status, "gpt-4", &configs, &status_cache);
                add_model_status(&mut status, "gpt-3.5-turbo-16k", &configs, &status_cache);
            }
            LLMProvider::Anthropic => {
                add_model_status(&mut status, "claude-opus", &configs, &status_cache);
                add_model_status(&mut status, "claude-sonnet", &configs, &status_cache);
            }
            LLMProvider::TogetherAI => {
                add_model_status(&mut status, "codellama-70b", &configs, &status_cache);
                add_model_status(&mut status, "mistral-instruct", &configs, &status_cache);
            }
            LLMProvider::GeminiPro => {
                add_model_status(&mut status, "gemini-pro", &configs, &status_cache);
            }
            _ => {}
        }
    }

    Json(status)
}

fn add_model_status(
    status: &mut HashMap<String, ModelInfo>,
    model_name: &str,
    configs: &HashMap<String, ModelConfig>,
    status_cache: &HashMap<String, ModelStatus>,
) {
    let default_config = ModelConfig {
        temperature: 0.7,
        max_tokens: 2048,
        top_p: 1.0,
        frequency_penalty: 0.0,
        presence_penalty: 0.0,
    };

    let config = configs.get(model_name).cloned().unwrap_or(default_config);
    let model_status = status_cache
        .get(model_name)
        .cloned()
        .unwrap_or(ModelStatus::Available);

    status.insert(
        model_name.to_string(),
        ModelInfo {
            name: model_name.to_string(),
            status: model_status,
            config,
        },
    );
}