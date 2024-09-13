use std::time::Instant;

use crate::{
    agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
    chunking::text_document::{Position, Range},
};
use async_trait::async_trait;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoToDefinitionRequest {
    fs_file_path: String,
    editor_url: String,
    position: Position,
}

impl GoToDefinitionRequest {
    pub fn new(fs_file_path: String, editor_url: String, position: Position) -> Self {
        Self {
            fs_file_path,
            editor_url,
            position,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoToDefinitionResponse {
    definitions: Vec<DefinitionPathAndRange>,
}

impl GoToDefinitionResponse {
    pub fn definitions(self) -> Vec<DefinitionPathAndRange> {
        self.definitions
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DefinitionPathAndRange {
    fs_file_path: String,
    range: Range,
}

impl DefinitionPathAndRange {
    pub fn file_path(&self) -> &str {
        &self.fs_file_path
    }

    pub fn range(&self) -> &Range {
        &self.range
    }
}

pub struct LSPGoToDefinition {
    client: reqwest::Client,
}

impl LSPGoToDefinition {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Tool for LSPGoToDefinition {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.is_go_to_definition()?;
        let start = Instant::now();
        let editor_endpoint = context.editor_url.to_owned() + "/go_to_definition";
        let response = self
            .client
            .post(editor_endpoint)
            .body(serde_json::to_string(&context).map_err(|_e| ToolError::SerdeConversionFailed)?)
            .send()
            .await
            .map_err(|_e| ToolError::ErrorCommunicatingWithEditor)?;
        println!("gtd::invoke::elapsed({:?})", start.elapsed());
        let response: GoToDefinitionResponse = response
            .json()
            .await
            .map_err(|_e| ToolError::SerdeConversionFailed)?;

        Ok(ToolOutput::GoToDefinition(response))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        agentic::tool::{input::ToolInput, r#type::Tool},
        chunking::text_document::Position,
    };

    use super::LSPGoToDefinition;

    /// This test runs with a live editor, sometime later we can abstract this
    /// part out
    #[tokio::test]
    async fn test_lsp_invocation() {
        let input = ToolInput::GoToDefinition(super::GoToDefinitionRequest {
            fs_file_path: "/Users/skcd/scratch/sidecar/sidecar/src/bin/webserver.rs".to_owned(),
            editor_url: "http://localhost:42423".to_owned(),
            position: Position::new(144, 54, 0),
        });
        let lsp_go_to_definition = LSPGoToDefinition::new();
        let result = lsp_go_to_definition.invoke(input).await;
        println!("{:?}", result);
        assert!(false);
    }
}
