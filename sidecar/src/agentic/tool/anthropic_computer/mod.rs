use std::sync::Arc;

use crate::agentic::tool::Tool;
use llm_client::broker::LLMBroker;

mod operations;
mod types;

pub use operations::*;
pub use types::*;

#[derive(Debug, thiserror::Error)]
pub enum AnthropicComputerError {
    #[error("Failed to process file operation: {0}")]
    FileOperationError(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

pub struct AnthropicComputerTool {
    llm_client: Arc<LLMBroker>,
    lsp_open_file: Arc<Box<dyn Tool + Send + Sync>>,
}

impl AnthropicComputerTool {
    pub fn new(
        llm_client: Arc<LLMBroker>,
        lsp_open_file: Arc<Box<dyn Tool + Send + Sync>>,
    ) -> Self {
        Self {
            llm_client,
            lsp_open_file,
        }
    }

    pub async fn open_file(&self, path: String, editor_url: String) -> Result<String, AnthropicComputerError> {
        let input = ToolInput {
            action: "open_file".to_string(),
            data: serde_json::json!({
                "fs_file_path": path,
                "editor_url": editor_url,
            }),
        };

        self.invoke(input)
            .await
            .map(|output| output.data.to_string())
            .map_err(|e| AnthropicComputerError::FileOperationError(e.to_string()))
    }

    pub async fn view_file(&self, path: String, editor_url: String) -> Result<String, AnthropicComputerError> {
        let input = ToolInput {
            action: "view_file".to_string(),
            data: serde_json::json!({
                "fs_file_path": path,
                "editor_url": editor_url,
                "read_only": true,
            }),
        };

        self.invoke(input)
            .await
            .map(|output| output.data.to_string())
            .map_err(|e| AnthropicComputerError::FileOperationError(e.to_string()))
    }

    pub async fn edit_file(
        &self,
        path: String,
        editor_url: String,
        changes: String,
    ) -> Result<String, AnthropicComputerError> {
        let input = ToolInput {
            action: "edit_file".to_string(),
            data: serde_json::json!({
                "fs_file_path": path,
                "editor_url": editor_url,
                "changes": changes,
            }),
        };

        self.invoke(input)
            .await
            .map(|output| output.data.to_string())
            .map_err(|e| AnthropicComputerError::FileOperationError(e.to_string()))
    }
}
