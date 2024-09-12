use std::sync::Arc;

use async_trait::async_trait;
use llm_client::broker::LLMBroker;

use crate::agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool};

#[derive(Debug, Clone)]
pub struct SkillSelectorRequest {
    contents: String,
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
}

#[async_trait]
impl Tool for SkillBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.skill_selector()?;

        dbg!(&context);

        todo!();
    }
}
