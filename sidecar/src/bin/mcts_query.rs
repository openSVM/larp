use std::fs;
use std::path::PathBuf;
use clap::Parser;
use serde_json;
use sidecar::mcts::action_node::ActionNode;
use sidecar::agentic::symbol::{
    events::message_event::SymbolEventMessageProperties,
    events::input::SymbolEventRequestId,
    identifier::LLMProperties,
    ui_event::UIEventWithID,
};
use sidecar::agentic::tool::session::tool_use_agent::{ToolUseAgent, ToolUseAgentReasoningInput, ToolUseAgentProperties};
use llm_client::{
    clients::types::LLMType,
    provider::{LLMProvider, LLMProviderAPIKeys, OpenAIProvider},
    broker::LLMBroker,
};
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

#[derive(Parser, Debug)]
#[clap(name = "mcts_query")]
#[clap(about = "Query an MCTS JSON file with questions")]
struct Args {
    /// Path to the MCTS JSON file
    #[clap(short, long)]
    mcts_path: PathBuf,

    /// Question to ask about the MCTS trajectory
    #[clap(short, long)]
    question: String,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let args = Args::parse();

    // Read the JSON file
    let json_content = fs::read_to_string(args.mcts_path)?;
    
    // Deserialize into a Vector of ActionNodes
    let action_nodes: Vec<ActionNode> = serde_json::from_str(&json_content)?;

    // Create LLMBroker
    let llm_broker = LLMBroker::new().await?;
    let llm_broker = Arc::new(llm_broker);

    // Create tool use agent
    let tool_use_agent = ToolUseAgent::new(
        llm_broker,
        std::env::current_dir()?.to_string_lossy().to_string(),
        "linux".to_string(),
        "bash".to_string(),
        ToolUseAgentProperties::new(
            true,
            Some("mcts_query".to_string()),
            None,
        ),
    );

    // Create necessary components for SymbolEventMessageProperties
    let (ui_sender, _ui_receiver) = unbounded_channel::<UIEventWithID>();
    let request_id = SymbolEventRequestId::new(
        "mcts_query".to_string(),
        "mcts_query_root".to_string(),
    );
    let cancellation_token = CancellationToken::new();
    let api_key = LLMProviderAPIKeys::OpenAI(OpenAIProvider::new(
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set")
    ));
    let llm_properties = LLMProperties::new(
        LLMType::O1,
        LLMProvider::OpenAI,
        api_key,
    );

    // Create message properties
    let message_properties = SymbolEventMessageProperties::new(
        request_id,
        ui_sender,
        "mcts_query".to_string(),
        cancellation_token,
        llm_properties,
    );

    // Create reasoning input
    let reasoning_input = ToolUseAgentReasoningInput::new(
        args.question.clone(),
        action_nodes,
        None,
        message_properties,
    );

    // Process the reasoning request
    let result = tool_use_agent.reasoning_output(reasoning_input).await?;
    
    // Print the result - use the accessor methods
    println!("Instruction:\n{}\n", result.instruction());
    println!("Notes:\n{}\n", result.notes());
    // Since there's no accessor for plan, we'll need to handle it differently
    // or just show the available information
    println!("Result summary:\nInstructions and notes from MCTS trajectory analysis");

    Ok(())
}