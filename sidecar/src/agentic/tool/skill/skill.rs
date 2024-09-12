use std::sync::Arc;

use async_trait::async_trait;
use futures::channel::mpsc::UnboundedSender;
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
pub struct SkillSelectorRequest {
    contents: String,
    root_request_id: String,
    _ui_sender: UnboundedSender<UIEventWithID>,
    _editor_url: String,
}

impl SkillSelectorRequest {
    pub fn new(
        contents: String,
        root_request_id: String,
        ui_sender: UnboundedSender<UIEventWithID>,
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
}

// or call it skill?
pub struct Skill {}

pub struct SkillBroker {
    llm_client: Arc<LLMBroker>,
}

impl SkillBroker {
    pub fn new(llm_client: Arc<LLMBroker>) -> Self {
        Self { llm_client }
    }

    pub fn system_message(&self) -> String {
        r#"system message"#.to_owned()
    }

    pub fn user_message(&self) -> String {
        r#"user message"#.to_owned()
    }
}

#[async_trait]
impl Tool for SkillBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.skill_selector()?;

        let system_message = LLMClientMessage::system(self.system_message());
        let user_message = LLMClientMessage::user(self.user_message());
        let root_request_id = context.root_request_id();

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
                    ("event_type".to_owned(), "skill selection".to_owned()),
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
