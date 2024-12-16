use async_trait::async_trait;
use serde_json::json;
use tokio::sync::mpsc;

use crate::agentic::tool::{Tool, ToolError, ToolOutput};
use crate::agentic::tool::code_edit::stream::{StreamProcessor, StreamUpdate};

use super::{AnthropicComputerRequest, AnthropicComputerResponse, FileOperation};

pub struct AnthropicStreamProcessor {
    request: AnthropicComputerRequest,
    content_buffer: String,
    language: Option<String>,
    error: Option<String>,
    tx: mpsc::Sender<StreamUpdate>,
}

impl AnthropicStreamProcessor {
    pub fn new(request: AnthropicComputerRequest, tx: mpsc::Sender<StreamUpdate>) -> Self {
        Self {
            request,
            content_buffer: String::new(),
            language: None,
            error: None,
            tx,
        }
    }

    async fn send_update(&mut self) -> Result<(), ToolError> {
        let response = AnthropicComputerResponse {
            content: self.content_buffer.clone(),
            language: self.language.clone(),
            error: self.error.clone(),
        };

        self.tx.send(StreamUpdate {
            data: json!(response),
            metadata: None,
            is_final: false,
        }).await.map_err(|e| ToolError::Other(format!("Failed to send stream update: {}", e)))?;

        Ok(())
    }
}

#[async_trait]
impl StreamProcessor for AnthropicStreamProcessor {
    async fn process_chunk(&mut self, chunk: String) -> Result<(), ToolError> {
        match self.request.operation {
            FileOperation::Open | FileOperation::View => {
                // For open/view operations, accumulate content
                self.content_buffer.push_str(&chunk);
                self.send_update().await?;
            },
            FileOperation::Edit => {
                // For edit operations, process changes incrementally
                if let Some(changes) = &self.request.changes {
                    self.content_buffer.push_str(&chunk);
                    // Process changes will be expanded in streaming implementation
                    self.send_update().await?;
                }
            },
        }

        Ok(())
    }

    async fn finalize(mut self) -> Result<ToolOutput, ToolError> {
        // Send final update
        let final_response = AnthropicComputerResponse {
            content: self.content_buffer,
            language: self.language,
            error: self.error,
        };

        self.tx.send(StreamUpdate {
            data: json!(final_response),
            metadata: None,
            is_final: true,
        }).await.map_err(|e| ToolError::Other(format!("Failed to send final update: {}", e)))?;

        Ok(ToolOutput {
            data: json!(final_response),
            metadata: None,
        })
    }
}
