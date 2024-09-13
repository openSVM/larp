use std::sync::Arc;

use async_trait::async_trait;
use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage, LLMType},
    provider::{AnthropicAPIKey, LLMProvider, LLMProviderAPIKeys},
};

use crate::agentic::{
    symbol::{events::message_event::SymbolEventMessageProperties, ui_event::UIEventWithID},
    tool::{editor, errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
};

#[derive(Debug, Clone)]
pub struct GoDefinitionEvaluatorRequest {
    contents: String,
    message_properties: SymbolEventMessageProperties,
}

impl GoDefinitionEvaluatorRequest {
    /// Creates a new `GoDefinitionEvaluatorRequest` instance.
    ///
    /// # Arguments
    ///
    /// * `contents` - The content string to be evaluated.
    /// * `message_properties` - The properties of the symbol event message.
    pub fn new(contents: String, message_properties: SymbolEventMessageProperties) -> Self {
        Self {
            contents,
            message_properties,
        }
    }

    /// Returns a reference to the contents of the request.
    pub fn contents(&self) -> &str {
        &self.contents
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
        r#"Your job is to go to a definition. Decide against which symbol this would be most useful to a given task."#
        .to_owned()
    }

    pub fn user_message(&self, request: GoDefinitionEvaluatorRequest) -> String {
        format!(r#"Session scratch pad:\n{}"#, request.contents()).to_owned()
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
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

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
            .await;

        dbg!(response);

        todo!();
    }
}
