use std::sync::Arc;

use async_trait::async_trait;
use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage, LLMType},
    provider::{AnthropicAPIKey, LLMProvider, LLMProviderAPIKeys},
};
use serde::Deserialize;
use serde_xml_rs::from_str;

use crate::agentic::{
    symbol::events::message_event::SymbolEventMessageProperties,
    tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
};

#[derive(Debug, Deserialize, Clone)]
#[serde(rename = "response")]
pub struct GoDefinitionEvaluatorResponse {
    #[serde(rename = "action", default)]
    pub actions: Vec<Action>,
}

impl GoDefinitionEvaluatorResponse {
    pub fn actions(&self) -> &[Action] {
        &self.actions.as_slice()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Action {
    pub line_contents: String,
    pub name: String,
}

impl Action {
    pub fn line_contents(&self) -> &str {
        &self.line_contents
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone)]
pub struct GoDefinitionEvaluatorRequest {
    pad_contents: String,
    file_contents: String,
    message_properties: SymbolEventMessageProperties,
}

impl GoDefinitionEvaluatorRequest {
    /// Creates a new `GoDefinitionEvaluatorRequest` instance.
    ///
    /// # Arguments
    ///
    /// * `contents` - The content string to be evaluated.
    /// * `message_properties` - The properties of the symbol event message.
    pub fn new(
        pad_contents: String,
        file_contents: String,
        message_properties: SymbolEventMessageProperties,
    ) -> Self {
        Self {
            pad_contents,
            file_contents,
            message_properties,
        }
    }

    /// Returns a reference to the contents of the request.
    pub fn pad_contents(&self) -> &str {
        &self.pad_contents
    }

    pub fn file_contents(&self) -> &str {
        &self.file_contents
    }

    /// Returns a reference to the message properties of the request.
    pub fn message_properties(&self) -> &SymbolEventMessageProperties {
        &self.message_properties
    }
}

pub struct GoDefinitionEvaluatorBroker {
    llm_client: Arc<LLMBroker>,
}

impl GoDefinitionEvaluatorBroker {
    pub fn new(llm_client: Arc<LLMBroker>) -> Self {
        Self { llm_client }
    }

    pub fn system_message(&self) -> String {
        r#"Your job is to go to a definition. Decide against which symbol this would be most useful for a given task list.

Consider the list of tasks in <tasks>

Format - print the line for the Symbol you want the go-to-definition for, along with the Symbol's name.

Example:
<response>
<action>
<line_contents>
pub struct Tag {
</line_contents>
<name>
Tag
</name>
</action>
</response>

Do not respond with anything other than the XML. Root tag: <response>
        "#
        .to_owned()
    }

    pub fn user_message(&self, request: GoDefinitionEvaluatorRequest) -> String {
        format!(
            r#"Session scratch pad:
{}
        
File contents:
{}
        "#,
            request.pad_contents(),
            request.file_contents()
        )
        .to_owned()
    }
}

#[async_trait]
impl Tool for GoDefinitionEvaluatorBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.go_definition_evaluator()?;
        let message_properties = context.message_properties();
        let root_request_id = message_properties.root_request_id();

        let system_message = LLMClientMessage::system(self.system_message());
        let user_message = LLMClientMessage::user(self.user_message(context.to_owned()));

        let request = LLMClientCompletionRequest::new(
            LLMType::ClaudeSonnet,
            vec![system_message, user_message],
            0.2,
            None,
        );

        let api_key = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("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".to_owned()));
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();

        let response = self
            .llm_client
            .stream_completion(
                api_key,
                request,
                LLMProvider::Anthropic,
                vec![
                    ("root_id".to_owned(), root_request_id.to_owned()),
                    ("event_type".to_owned(), "evaluate_go_definition".to_owned()),
                ]
                .into_iter()
                .collect(),
                sender,
            )
            .await
            .map_err(|e| ToolError::LLMClientError(e))?;

        let parsed = from_str::<GoDefinitionEvaluatorResponse>(&response).map_err(|e| {
            eprintln!("{:?}", e);
            ToolError::SerdeConversionFailed
        })?;

        Ok(ToolOutput::GoDefinitionEvaluator(parsed))
    }
}
