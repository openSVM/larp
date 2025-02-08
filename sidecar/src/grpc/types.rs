use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone)]
pub struct UserContext {
    pub repo_ref: String,
    pub precise_context: Vec<PreciseContext>,
    pub cursor_position: Option<CursorPosition>,
    pub current_view_port: Option<ViewPort>,
    pub language: String,
}

#[derive(Debug, Clone)]
pub struct AgentSessionRequest {
    pub session_id: String,
    pub exchange_id: String,
    pub editor_url: String,
    pub query: String,
    pub user_context: Option<UserContext>,
    pub repo_ref: Option<RepoRef>,
    pub root_directory: String,
    pub project_labels: Vec<String>,
    pub codebase_search: bool,
    pub access_token: String,
    pub model_configuration: Option<LLMClientConfig>,
    pub all_files: Vec<String>,
    pub open_files: Vec<String>,
    pub shell: String,
    pub aide_rules: Option<String>,
    pub reasoning: bool,
    pub semantic_search: bool,
    pub is_devtools_context: bool,
}

#[derive(Debug, Clone)]
pub enum AgentResponseType {
    Thinking(String),
    Action(String),
    Error(Error),
}

#[derive(Debug, Clone)]
pub struct AgentResponse {
    pub response: AgentResponseType,
}

#[derive(Debug, Clone)]
pub struct Error {
    pub message: String,
    pub kind: ErrorKind,
}

#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
    Unknown,
    User,
    NotFound,
    Configuration,
    UpstreamService,
    Internal,
}

#[derive(Debug, Clone)]
pub struct PreciseContext {
    pub symbol: Symbol,
    pub hover_text: Vec<String>,
    pub definition_snippet: DefinitionSnippet,
    pub fs_file_path: String,
    pub relative_file_path: String,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub fuzzy_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CursorPosition {
    pub start_position: Position,
    pub end_position: Position,
}

#[derive(Debug, Clone)]
pub struct ViewPort {
    pub start_position: Position,
    pub end_position: Position,
    pub relative_path: String,
    pub fs_file_path: String,
    pub text_on_screen: String,
}

#[derive(Debug, Clone)]
pub struct DefinitionSnippet {
    pub context: String,
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Debug, Clone)]
pub struct RepoRef {
    pub repo_name: String,
    pub branch: String,
    pub commit_hash: String,
}

#[derive(Debug, Clone)]
pub struct LLMClientConfig {
    pub model_type: String,
    pub provider: String,
    pub config: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct AgentEditRequest {
    pub file_path: String,
    pub content: String,
    pub edit_range: Range,
    pub context: Option<UserContext>,
}

#[derive(Debug, Clone)]
pub struct AgentEditResponse {
    pub edited_content: String,
    pub modified_range: Option<Range>,
}

#[derive(Debug, Clone)]
pub struct ToolUseRequest {
    pub tool_name: String,
    pub parameters: std::collections::HashMap<String, String>,
    pub context: Option<UserContext>,
}

#[derive(Debug, Clone)]
pub struct ToolUseResponse {
    pub result: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EditFileRequest {
    pub file_path: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct EditFileResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DocumentationRequest {
    pub file_content: String,
    pub language: String,
}

#[derive(Debug, Clone)]
pub struct DocumentationResponse {
    pub documentation_strings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TreeSitterValidationRequest {
    pub content: String,
    pub language: String,
}

#[derive(Debug, Clone)]
pub struct TreeSitterValidationResponse {
    pub is_valid: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProbeStopRequest {
    pub request_id: String,
}

#[derive(Debug, Clone)]
pub struct ProbeStopResponse {
    pub done: bool,
}

#[derive(Debug, Clone)]
pub struct CodeSculptingRequest {
    pub request_id: String,
    pub instruction: String,
}

#[derive(Debug, Clone)]
pub struct CodeSculptingHealRequest {
    pub request_id: String,
}

#[derive(Debug, Clone)]
pub struct CodeSculptingHealResponse {
    pub done: bool,
}

#[derive(Debug, Clone)]
pub struct AgenticDiagnosticsRequest {
    pub fs_file_path: String,
    pub diagnostics: Vec<DiagnosticData>,
    pub source: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DiagnosticData {
    pub message: String,
    pub range: Range,
    pub range_content: String,
}

#[derive(Debug, Clone)]
pub struct AgenticDiagnosticsResponse {
    pub done: bool,
}

#[derive(Debug, Clone)]
pub struct SweBenchRequest {
    pub git_dname: String,
    pub problem_statement: String,
    pub editor_url: String,
    pub test_endpoint: String,
    pub repo_map_file: Option<String>,
    pub gcloud_access_token: String,
    pub swe_bench_id: String,
}

#[derive(Debug, Clone)]
pub struct SweBenchResponse {
    pub done: bool,
}

#[derive(Debug, Clone)]
pub struct VerifyModelConfigRequest {
    pub model_configuration: LLMClientConfig,
}

#[derive(Debug, Clone)]
pub struct VerifyModelConfigResponse {
    pub valid: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CancelExchangeRequest {
    pub exchange_id: String,
    pub session_id: String,
    pub editor_url: String,
    pub access_token: String,
    pub model_configuration: LLMClientConfig,
}

#[derive(Debug, Clone)]
pub struct FeedbackExchangeRequest {
    pub exchange_id: String,
    pub session_id: String,
    pub step_index: Option<u32>,
    pub editor_url: String,
    pub accepted: bool,
    pub access_token: String,
    pub model_configuration: LLMClientConfig,
}

#[derive(Debug, Clone)]
pub struct SessionUndoRequest {
    pub session_id: String,
    pub exchange_id: String,
}

#[derive(Debug, Clone)]
pub struct SessionUndoResponse {
    pub done: bool,
}

// Add conversion traits between tonic types and our types
#[cfg(feature = "grpc")]
pub mod conversions {
    use super::*;
    use tonic::Response;
    
    impl From<Position> for crate::grpc::proto::Position {
        fn from(pos: Position) -> Self {
            Self {
                line: pos.line,
                character: pos.character,
            }
        }
    }

    impl From<crate::grpc::proto::Position> for Position {
        fn from(pos: crate::grpc::proto::Position) -> Self {
            Self {
                line: pos.line,
                character: pos.character,
            }
        }
    }

    // Add similar From implementations for other types
    // This allows easy conversion between our types and tonic-generated types
}