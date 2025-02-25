pub mod agent;
pub mod agent_stream;
pub mod agentic;
pub mod config;
pub mod context_trimming;
pub mod file_edit;
pub mod health;
pub mod in_line_agent;
pub mod in_line_agent_stream;
pub mod inline_completion;
pub mod model_selection;
pub mod plan;
pub mod tree_sitter;
pub mod types;

use axum::Router;
use std::sync::Arc;
use crate::{
    auth, fs, lsp, cache, metrics, plugins, security, models,
    webserver::{
        health, config, tree_sitter, agent, agentic,
    },
};

pub async fn create_router() -> anyhow::Result<Router> {
    // Initialize ModelState
    let model_state = Arc::new(models::ModelState::new().await?);
    model_state.initialize_default_configs().await;

    Ok(Router::new()
        // Existing routes
        .merge(health::router())
        .merge(config::router())
        .merge(tree_sitter::router())
        .merge(agent::router())
        .merge(agentic::router())
        // New independent routes
        .merge(auth::router())
        .merge(fs::router())
        .merge(lsp::router())
        .merge(cache::router())
        .merge(metrics::router())
        .merge(plugins::router())
        .merge(security::router())
        .merge(models::router().with_state(model_state)))
}