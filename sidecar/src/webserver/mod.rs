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
use crate::{
    auth, fs, lsp, cache, metrics, plugins, security,
    webserver::{
        health, config, tree_sitter, agent, agentic,
    },
};

pub fn router() -> Router {
    Router::new()
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
}