use std::sync::Arc;

use async_trait::async_trait;
use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage, LLMType},
    provider::{AnthropicAPIKey, LLMProvider, LLMProviderAPIKeys},
};

use crate::agentic::{
    symbol::ui_event::UIEventWithID,
    tool::{editor, errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
};

#[derive(Debug, Clone)]
pub struct GoDefinitionEvaluatorRequest {
    contents: String,
    root_request_id: String,
    _ui_sender: tokio::sync::mpsc::UnboundedSender<UIEventWithID>,
    _editor_url: String,
}

impl GoDefinitionEvaluatorRequest {
    pub fn new(
        contents: String,
        root_request_id: String,
        ui_sender: tokio::sync::mpsc::UnboundedSender<UIEventWithID>,
        editor_url: String,
    ) -> Self {
        Self {
            contents,
            root_request_id,
            _ui_sender: ui_sender,
            _editor_url: editor_url,
        }
    }

    pub fn root_request_id(&self) -> &str {
        &self.root_request_id
    }

    pub fn contents(&self) -> &str {
        &self.contents
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
        r#"Based on provided information overview of a coding session, you must select a tool to use, and provide necessary arguments/parameters with which to use them
Tools available:
- Go to Definition
- Go to References
- Keyword Search
- Make edits
- Ask question
        "#
        .to_owned()
    }

    pub fn user_message(&self, request: GoDefinitionEvaluatorRequest) -> String {
        format!(r#"Coding Session scratch pad:\n{}"#, request.contents()).to_owned()
    }
}

#[async_trait]
impl Tool for GoDefinitionEvaluatorBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.skill_selector()?;
        let root_request_id = context.root_request_id();

        let system_message = LLMClientMessage::system(self.system_message());
        let user_message = LLMClientMessage::user(self.user_message(context.to_owned()));

        dbg!(&user_message);

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
                    ("event_type".to_owned(), "skill_selection".to_owned()),
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
