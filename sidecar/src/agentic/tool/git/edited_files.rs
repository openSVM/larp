//! The edited files and the git-diff which is ordered by timestamp
//! The idea is that the file which we are editing can go last

use crate::agentic::tool::{
    errors::ToolError,
    helpers::diff_recent_changes::DiffFileContent,
    input::ToolInput,
    output::ToolOutput,
    r#type::{Tool, ToolRewardScale},
};
use async_trait::async_trait;
use logging::new_client;

/// Extracts operation ID from PR description or title
/// Returns None if no operation ID is found
fn extract_operation_id(content: &str) -> Option<String> {
    // Common patterns for operation IDs in PR titles or descriptions
    let patterns = [
        // Look for explicit operation ID format: "operation_id: XXX" or "operationId: XXX"
        r"operation[_\s]?[iI]d:\s*([a-zA-Z0-9_-]+)",
        // Look for operation ID in brackets: [XXX]
        r"\[([a-zA-Z0-9_-]+)\]",
        // Look for operation ID in parentheses: (XXX)
        r"\(([a-zA-Z0-9_-]+)\)",
        // Look for operation ID with prefix: op-XXX
        r"op-([a-zA-Z0-9_-]+)",
    ];

    for pattern in patterns {
        if let Some(captures) = regex::Regex::new(pattern).ok()?.captures(content) {
            if let Some(id) = captures.get(1) {
                return Some(id.as_str().to_string());
            }
        }
    }

    None
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EditedFilesRequest {
    editor_url: String,
    diff_file_content: Vec<DiffFileContent>,
}

impl EditedFilesRequest {
    pub fn new(editor_url: String, diff_file_content: Vec<DiffFileContent>) -> Self {
        Self {
            editor_url,
            diff_file_content,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EditedGitDiffFile {
    fs_file_path: String,
    diff: String,
    current_content: String,
    updated_timestamp_ms: i64,
    #[serde(default)]
    operation_id: Option<String>,
}

impl EditedGitDiffFile {
    pub fn fs_file_path(&self) -> &str {
        &self.fs_file_path
    }

    pub fn diff(&self) -> &str {
        &self.diff
    }

    pub fn updated_timestamp_ms(&self) -> i64 {
        self.updated_timestamp_ms
    }

    pub fn current_content(&self) -> &str {
        &self.current_content
    }
    
    pub fn operation_id(&self) -> Option<&str> {
        self.operation_id.as_deref()
    }
    
    /// Tries to extract operation ID from diff or current content if not already set
    pub fn extract_operation_id(&mut self) {
        if self.operation_id.is_none() {
            self.operation_id = extract_operation_id(&self.diff)
                .or_else(|| extract_operation_id(&self.current_content));
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EditedFilesResponse {
    changed_files: Vec<EditedGitDiffFile>,
    #[serde(default)]
    operation_id: Option<String>,
}

impl EditedFilesResponse {
    pub fn changed_files(mut self) -> Vec<EditedGitDiffFile> {
        // Extract operation IDs from files if not already set
        for file in &mut self.changed_files {
            file.extract_operation_id();
        }
        self.changed_files
    }
    
    pub fn operation_id(&self) -> Option<&str> {
        self.operation_id.as_deref()
    }
    
    /// Tries to find an operation ID from any of the changed files
    pub fn extract_operation_id(&mut self) -> Option<String> {
        // First check if we already have an operation ID
        if let Some(id) = &self.operation_id {
            return Some(id.clone());
        }
        
        // Try to extract from any of the changed files
        for file in &mut self.changed_files {
            file.extract_operation_id();
            if let Some(id) = file.operation_id() {
                self.operation_id = Some(id.to_string());
                return self.operation_id.clone();
            }
        }
        
        None
    }
}

pub struct EditedFiles {
    client: reqwest_middleware::ClientWithMiddleware,
}

impl EditedFiles {
    pub fn new() -> Self {
        Self {
            client: new_client(),
        }
    }
}

#[async_trait]
impl Tool for EditedFiles {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.should_edited_files()?;
        let editor_endpoint = context.editor_url.to_owned() + "/recent_edits";
        let response = self
            .client
            .post(editor_endpoint)
            .body(serde_json::to_string(&context).map_err(|_e| ToolError::SerdeConversionFailed)?)
            .send()
            .await
            .map_err(|_e| ToolError::ErrorCommunicatingWithEditor)?;
        let mut response: EditedFilesResponse = response.json().await.map_err(|e| {
            eprintln!("edited_files::{:?}", &e);
            ToolError::SerdeConversionFailed
        })?;
        
        // Try to extract operation ID if not already set
        response.extract_operation_id();
        
        Ok(ToolOutput::edited_files(response))
    }

    fn tool_description(&self) -> String {
        "".to_owned()
    }

    fn tool_input_format(&self) -> String {
        "".to_owned()
    }

    fn get_evaluation_criteria(&self, _trajectory_length: usize) -> Vec<String> {
        vec![]
    }

    fn get_reward_scale(&self, _trajectory_length: usize) -> Vec<ToolRewardScale> {
        vec![]
    }
}