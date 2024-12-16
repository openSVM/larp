use async_trait::async_trait;
use std::sync::Arc;

use crate::agentic::tool::{Tool, ToolError, ToolInput, ToolOutput};
use crate::agentic::tool::lsp::open_file::LSPOpenFileRequest;
use crate::agentic::tool::code_edit::search_and_replace::SearchAndReplaceEditingRequest;

use super::AnthropicComputerTool;

#[async_trait]
impl Tool for AnthropicComputerTool {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let request = match input.action.as_str() {
            "open_file" => {
                // Delegate to LSPOpenFile for file opening
                let open_request: LSPOpenFileRequest = serde_json::from_value(input.data.clone())
                    .map_err(|e| ToolError::InvalidInput(format!("Failed to parse open file request: {}", e)))?;

                self.lsp_open_file.invoke(ToolInput {
                    action: "open".to_string(),
                    data: serde_json::to_value(open_request)
                        .map_err(|e| ToolError::InvalidInput(format!("Failed to serialize open request: {}", e)))?,
                }).await?
            },
            "view_file" => {
                // Similar to open_file but with read-only flag
                let mut open_request: LSPOpenFileRequest = serde_json::from_value(input.data.clone())
                    .map_err(|e| ToolError::InvalidInput(format!("Failed to parse view file request: {}", e)))?;
                open_request.read_only = true;

                self.lsp_open_file.invoke(ToolInput {
                    action: "open".to_string(),
                    data: serde_json::to_value(open_request)
                        .map_err(|e| ToolError::InvalidInput(format!("Failed to serialize view request: {}", e)))?,
                }).await?
            },
            "edit_file" => {
                // Use SearchAndReplaceEditing patterns for edit operations
                let edit_request: SearchAndReplaceEditingRequest = serde_json::from_value(input.data.clone())
                    .map_err(|e| ToolError::InvalidInput(format!("Failed to parse edit request: {}", e)))?;

                // Process edit request using Anthropic capabilities
                // This will be expanded in the streaming support step
                ToolOutput::default()
            },
            _ => return Err(ToolError::InvalidInput(format!("Unknown action: {}", input.action))),
        };

        Ok(request)
    }

    fn name(&self) -> String {
        "anthropic_computer".to_string()
    }

    fn description(&self) -> String {
        "Anthropic computer use tool for file operations".to_string()
    }
}
