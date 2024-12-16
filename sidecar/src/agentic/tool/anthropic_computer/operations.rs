use async_trait::async_trait;
use serde_json::json;

use crate::agentic::tool::{Tool, ToolError, ToolInput, ToolOutput};
use crate::agentic::tool::lsp::open_file::LSPOpenFileRequest;

use super::{AnthropicComputerTool, AnthropicComputerRequest, AnthropicComputerResponse, FileOperation};

#[async_trait]
impl Tool for AnthropicComputerTool {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let request: AnthropicComputerRequest = serde_json::from_value(input.data.clone())
            .map_err(|e| ToolError::InvalidInput(format!("Failed to parse request: {}", e)))?;

        let response = match request.operation {
            FileOperation::Open | FileOperation::View => {
                // Delegate to LSPOpenFile for file opening/viewing
                let open_request = LSPOpenFileRequest {
                    fs_file_path: request.fs_file_path,
                    editor_url: request.editor_url,
                    read_only: request.operation == FileOperation::View,
                };

                let result = self.lsp_open_file.invoke(ToolInput {
                    action: "open".to_string(),
                    data: serde_json::to_value(open_request)
                        .map_err(|e| ToolError::InvalidInput(format!("Failed to serialize open request: {}", e)))?,
                }).await?;

                AnthropicComputerResponse {
                    content: result.data.to_string(),
                    language: None,
                    error: None,
                }
            },
            FileOperation::Edit => {
                // Process edit request using Anthropic capabilities
                // This will be expanded in the streaming support step
                AnthropicComputerResponse::default()
            },
        };

        Ok(ToolOutput {
            data: json!(response),
            metadata: None,
        })
    }

    fn name(&self) -> String {
        "anthropic_computer".to_string()
    }

    fn description(&self) -> String {
        "Anthropic computer use tool for file operations".to_string()
    }
}
