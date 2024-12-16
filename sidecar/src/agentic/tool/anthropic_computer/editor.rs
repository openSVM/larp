use async_trait::async_trait;
use serde_json::json;

use crate::agentic::tool::{Tool, ToolError, ToolInput, ToolOutput};
use crate::agentic::tool::code_edit::code_editor::{CodeEditorParameters, CodeEditorResponse};

use super::{AnthropicComputerRequest, AnthropicComputerResponse, FileOperation};

#[async_trait]
impl AnthropicComputerTool {
    pub(crate) async fn send_to_editor(&self, request: AnthropicComputerRequest) -> Result<ToolOutput, ToolError> {
        let editor_params = CodeEditorParameters {
            fs_file_path: request.fs_file_path.clone(),
            editor_url: request.editor_url.clone(),
            read_only: request.operation == FileOperation::View,
        };

        let editor_response: CodeEditorResponse = match request.operation {
            FileOperation::Open | FileOperation::View => {
                // Handle file opening/viewing through editor
                let response = self.lsp_open_file.invoke(ToolInput {
                    action: "open".to_string(),
                    data: json!({
                        "fs_file_path": editor_params.fs_file_path,
                        "editor_url": editor_params.editor_url,
                        "read_only": editor_params.read_only,
                    }),
                }).await?;

                serde_json::from_value(response.data)
                    .map_err(|e| ToolError::InvalidInput(format!("Failed to parse editor response: {}", e)))?
            },
            FileOperation::Edit => {
                if let Some(changes) = request.changes {
                    // Process edit request using Anthropic capabilities
                    // This will be expanded in the streaming support step
                    CodeEditorResponse {
                        content: changes,
                        language: None,
                    }
                } else {
                    return Err(ToolError::InvalidInput("Changes required for edit operation".to_string()));
                }
            },
        };

        Ok(ToolOutput {
            data: json!(AnthropicComputerResponse {
                content: editor_response.content,
                language: editor_response.language,
                error: None,
            }),
            metadata: None,
        })
    }
}
