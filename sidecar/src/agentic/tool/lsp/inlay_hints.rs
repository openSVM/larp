//! We can grab the inlay hints from the LSP using this

use crate::{
    agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
    chunking::text_document::{Position, Range},
};
use async_trait::async_trait;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InlayHintsRequest {
    fs_file_path: String,
    range: Range,
    editor_url: String,
}

impl InlayHintsRequest {
    pub fn new(fs_file_path: String, range: Range, editor_url: String) -> Self {
        Self {
            fs_file_path,
            range,
            editor_url,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InlayHintsResponseParts {
    position: Position,
    padding_left: bool,
    padding_right: bool,
    values: Vec<String>,
}

impl InlayHintsResponseParts {
    pub fn position(&self) -> &Position {
        &self.position
    }

    pub fn padding_left(&self) -> bool {
        self.padding_left
    }

    pub fn padding_right(&self) -> bool {
        self.padding_right
    }

    pub fn values(&self) -> &[String] {
        self.values.as_slice()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InlayHintsResponse {
    parts: Vec<InlayHintsResponseParts>,
}

impl InlayHintsResponse {
    pub fn parts(self) -> Vec<InlayHintsResponseParts> {
        self.parts
    }
}

pub struct InlayHints {
    client: reqwest::Client,
}

impl InlayHints {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Tool for InlayHints {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.inlay_hints_request()?;
        let editor_endpoint = context.editor_url.to_owned() + "/inlay_hints";
        let response = self
            .client
            .post(editor_endpoint)
            .body(serde_json::to_string(&context).map_err(|_e| ToolError::SerdeConversionFailed)?)
            .send()
            .await
            .map_err(|_e| ToolError::ErrorCommunicatingWithEditor)?;
        let response: InlayHintsResponse =
            dbg!(response.json().await).map_err(|_e| ToolError::SerdeConversionFailed)?;
        Ok(ToolOutput::inlay_hints(response))
    }
}
