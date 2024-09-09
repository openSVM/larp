//! The scratch pad broker here contains the information for iteration on the scratch pad
//! Right now the structure of the scratchpad I can come up with is this:
//! <task>
//! </task>
//! <start_condition>
//! </start_condition>
//! <changes_made>
//! </changes_made>
//! <insights>
//! </insights>
//! <signals>
//! </signals>
//! <next_steps>
//! </next_steps>
//! This is a big maybe for now tho, so we will revisit this in the future but want to give this a try

use std::sync::Arc;

use llm_client::broker::LLMBroker;

use crate::agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool};


/// Making all of this optional for now, so we can see how the LLM would perform
#[derive(Debug, Clone)]
struct ScratchPadDeltaInput {
    human_input: Option<String>,
    start_condition: Option<String>,
    changes_made: Option<String>,
    signals: Option<String>,
}

pub struct ScratchPadBroker {
    llm_client: Arc<LLMBroker>,
}

impl ScratchPadBroker {

}

#[async_trait]
impl Tool for ScratchPadBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        // figure out how to handle things over here
    }
}