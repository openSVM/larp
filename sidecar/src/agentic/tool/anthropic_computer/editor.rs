use async_trait::async_trait;
use serde_json::json;
use tokio::sync::mpsc;

use crate::agentic::tool::{Tool, ToolError, ToolInput, ToolOutput};
use crate::agentic::tool::code_edit::code_editor::{CodeEditorParameters, CodeEditorResponse};
use crate::agentic::tool::code_edit::stream::StreamUpdate;

use super::{AnthropicComputerRequest, AnthropicComputerResponse, FileOperation, AnthropicStreamProcessor};

#[async_trait]
impl AnthropicComputerTool {
    pub(crate) async fn send_to_editor(&self, request: AnthropicComputerRequest) -> Result<ToolOutput, ToolError> {
        let editor_params = CodeEditorParameters {
            fs_file_path: request.fs_file_path.clone(),
            editor_url: request.editor_url.clone(),
            read_only: request.operation == FileOperation::View,
        };

        // Create streaming channel
        let (tx, mut rx) = mpsc::channel::<StreamUpdate>(32);
        let mut processor = AnthropicStreamProcessor::new(request.clone(), tx);

        // Handle streaming updates
        tokio::spawn(async move {
            while let Some(update) = rx.recv().await {
                if update.is_final {
                    break;
                }
                // Process intermediate updates
            }
        });

        let editor_response: CodeEditorResponse = match request.operation {
            FileOperation::Open | FileOperation::View => {
                // Handle file opening/viewing through editor with streaming
                let response = self.lsp_open_file.invoke(ToolInput {
                    action: "open".to_string(),
                    data: json!({
                        "fs_file_path": editor_params.fs_file_path,
                        "editor_url": editor_params.editor_url,
                        "read_only": editor_params.read_only,
                    }),
                }).await?;

                // Process response through stream processor
                processor.process_chunk(response.data.to_string()).await?;
                let final_output = processor.finalize().await?;

                serde_json::from_value(final_output.data)
                    .map_err(|e| ToolError::InvalidInput(format!("Failed to parse editor response: {}", e)))?
            },
            FileOperation::Edit => {
                if let Some(changes) = request.changes {
                    // Process edit request using Anthropic capabilities with streaming
                    processor.process_chunk(changes.clone()).await?;
                    let final_output = processor.finalize().await?;

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
