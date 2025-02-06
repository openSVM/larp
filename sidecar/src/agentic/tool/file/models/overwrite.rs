use std::sync::Arc;
use reqwest::Client;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::agentic::tool::{
    errors::ToolError,
    input::ToolInput,
    output::ToolOutput,
    r#type::{Tool, ToolRewardScale},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct OverwriteFileRequest {
    pub fs_file_path: String,
    pub updated_content: String,
}

pub struct FileOverwrite {
    client: Arc<Client>,
    base_url: String,
}

impl FileOverwrite {
    pub fn new(client: Arc<Client>, base_url: String) -> Self {
        Self { client, base_url }
    }
}

#[async_trait]
impl Tool for FileOverwrite {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let request = match input {
            ToolInput::OverwriteFile(req) => req,
            _ => return Err(ToolError::InvalidToolInput),
        };

        let url = format!("{}/api/file/apply_edits", self.base_url);
        
        let response = self.client
            .post(&url)
            .json(&serde_json::json!({
                "fs_file_path": request.fs_file_path,
                "edited_content": request.updated_content,
                "apply_directly": true
            }))
            .send()
            .await
            .map_err(|_| ToolError::ErrorCommunicatingWithEditor)?;

        if !response.status().is_success() {
            return Err(ToolError::ErrorCommunicatingWithEditor);
        }

        Ok(ToolOutput::Success)
    }

    fn tool_description(&self) -> String {
        "Overwrites the content of a file with new content".to_string()
    }

    fn tool_input_format(&self) -> String {
        r#"Parameters:
- fs_file_path: (required) The absolute path of the file to overwrite
- updated_content: (required) The new content to write to the file

Usage:
<overwrite_file>
<fs_file_path>path/to/file</fs_file_path>
<updated_content>new content</updated_content>
</overwrite_file>"#.to_string()
    }

    fn get_evaluation_criteria(&self, _trajectory_length: usize) -> Vec<String> {
        vec![
            "File path exists and is valid".to_string(),
            "Content is properly formatted".to_string(),
            "Operation completed successfully".to_string(),
        ]
    }

    fn get_reward_scale(&self, _trajectory_length: usize) -> Vec<ToolRewardScale> {
        vec![
            ToolRewardScale::new(75, 100, "File overwritten successfully with valid content"),
            ToolRewardScale::new(0, 74, "Operation failed or content was invalid"),
        ]
    }
}