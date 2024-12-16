use async_trait::async_trait;
use serde_json::json;

use crate::agentic::tool::{Tool, ToolError, ToolInput, ToolOutput};

use super::{AnthropicComputerTool, AnthropicComputerRequest};

#[async_trait]
impl Tool for AnthropicComputerTool {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let request: AnthropicComputerRequest = serde_json::from_value(input.data.clone())
            .map_err(|e| ToolError::InvalidInput(format!("Failed to parse request: {}", e)))?;

        // Delegate to editor communication implementation
        self.send_to_editor(request).await
    }

    fn name(&self) -> String {
        "anthropic_computer".to_string()
    }

    fn description(&self) -> String {
        "Anthropic computer use tool for file operations".to_string()
    }
}
