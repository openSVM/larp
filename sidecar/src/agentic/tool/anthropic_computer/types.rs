use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileOperation {
    Open,
    View,
    Edit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicComputerRequest {
    pub operation: FileOperation,
    pub fs_file_path: String,
    pub editor_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changes: Option<String>,
    #[serde(default)]
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicComputerResponse {
    pub content: String,
    pub language: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicEditRequest {
    pub fs_file_path: String,
    pub editor_url: String,
    pub changes: String,
    pub context: Option<String>,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicEditResponse {
    pub content: String,
    pub applied_changes: Vec<String>,
    pub error: Option<String>,
}

impl Default for AnthropicComputerResponse {
    fn default() -> Self {
        Self {
            content: String::new(),
            language: None,
            error: None,
        }
    }
}
