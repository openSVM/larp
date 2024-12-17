use anyhow::Result;
use clap::Parser;
use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    config::LLMBrokerConfiguration,
    provider::{
        AnthropicAPIKey, GoogleAIStudioKey, LLMProvider, LLMProviderAPIKeys, OpenRouterAPIKey,
    },
};
use sidecar::{
    agentic::{
        symbol::{
            events::{input::SymbolEventRequestId, message_event::SymbolEventMessageProperties},
            identifier::LLMProperties,
            tool_box::ToolBox,
        },
        tool::{
            broker::{ToolBroker, ToolBrokerConfiguration},
            code_edit::models::broker::CodeEditBroker,
            r#type::ToolType,
        },
    },
    chunking::{editor_parsing::EditorParsing, languages::TSLanguageParsing},
    inline_completion::symbols_tracker::SymbolTrackerInline,
    mcts::{
        action_node::SearchTree, agent_settings::settings::AgentSettings,
        selector::selector::Selector,
    },
};
use std::path::PathBuf;
use std::sync::Arc;

/// Define the command-line arguments
#[derive(Parser, Debug)]
#[command(author = "skcd", version = "1.0", about = "Midwit tool use")]
struct CliArgs {
    /// Timeout in seconds
    #[arg(long)]
    timeout: usize,

    /// Repository location
    #[arg(long)]
    repo_location: PathBuf,

    /// Repository name (I am sorry for asking this)
    #[arg(long)]
    repo_name: String,

    /// Anthropic api key
    #[arg(long, default_value = None)]
    anthropic_api_key: Option<String>,

    /// OPen Router api key
    #[arg(long, default_value = None)]
    openrouter_api_key: Option<String>,

    /// The run id for the current run
    #[arg(long)]
    problem_statement: String,
}

fn default_index_dir() -> PathBuf {
    match directories::ProjectDirs::from("ai", "codestory", "sidecar") {
        Some(dirs) => dirs.data_dir().to_owned(),
        None => "codestory_sidecar".into(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = CliArgs::parse();

    let repo_locations = vec![
        PathBuf::from("/Users/zi/codestory/website/"),
        PathBuf::from("/Users/zi/codestory/website_clones/clone_1"),
        PathBuf::from("/Users/zi/codestory/website_clones/clone_2"),
    ];

    let handles: Vec<_> = repo_locations
        .into_iter()
        .map(|repo_location| {
            let problem_statement = args.problem_statement.clone();
            let anthropic_api_key = args.anthropic_api_key.clone();
            let openrouter_api_key = args.openrouter_api_key.clone();
            let timeout = args.timeout;
            let repo_name = args.repo_name.clone();

            tokio::spawn(async move {
                let editor_parsing = Arc::new(EditorParsing::default());
                let symbol_broker = Arc::new(SymbolTrackerInline::new(editor_parsing.clone()));
                let llm_broker = Arc::new(
                    LLMBroker::new(LLMBrokerConfiguration::new(default_index_dir()))
                        .await
                        .expect("to initialize properly"),
                );
                let tool_broker = Arc::new(ToolBroker::new(
                    llm_broker.clone(),
                    Arc::new(CodeEditBroker::new()),
                    symbol_broker.clone(),
                    Arc::new(TSLanguageParsing::init()),
                    ToolBrokerConfiguration::new(None, true),
                    LLMProperties::new(
                        LLMType::GeminiPro,
                        LLMProvider::GoogleAIStudio,
                        LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new("".to_owned())),
                    ),
                ));

                let tool_box = Arc::new(ToolBox::new(tool_broker, symbol_broker, editor_parsing));

                let editor_url = "".to_owned();
                let run_id = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("clocks to not drift")
                    .as_secs()
                    .to_string();

                let log_directory;
                {
                    let log_directory_path = default_index_dir().join("tool_use");
                    if tokio::fs::metadata(&log_directory_path).await.is_err() {
                        tokio::fs::create_dir(&log_directory_path)
                            .await
                            .expect("directory creation to not fail");
                    }
                    log_directory = default_index_dir().join("tool_use").join(run_id.to_owned());
                }

                let model_configuration: LLMProperties;
                if let Some(anthropic_key) = anthropic_api_key {
                    model_configuration = LLMProperties::new(
                        LLMType::ClaudeSonnet,
                        LLMProvider::Anthropic,
                        LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new(anthropic_key)),
                    );
                } else if let Some(open_router_key) = openrouter_api_key {
                    model_configuration = LLMProperties::new(
                        LLMType::ClaudeSonnet,
                        LLMProvider::OpenRouter,
                        LLMProviderAPIKeys::OpenRouter(OpenRouterAPIKey::new(open_router_key)),
                    );
                } else {
                    println!("NO VALID KEY FOUND, TERMINATING");
                    return Ok::<(), Box<dyn std::error::Error + Send + Sync>>(());
                }

                let session_id = format!("{}", run_id.to_string());
                println!("session_id:{}", &session_id);

                let initial_exchange_id = 0;
                let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
                let cancellation_token = tokio_util::sync::CancellationToken::new();
                let message_properties = SymbolEventMessageProperties::new(
                    SymbolEventRequestId::new(
                        initial_exchange_id.to_string().to_owned(),
                        run_id.to_string(),
                    ),
                    sender.clone(),
                    editor_url,
                    cancellation_token.clone(),
                    model_configuration,
                );

                let agent_settings = AgentSettings::new(true, true);
                let bad_actions = vec![ToolType::CodeEditorTool];
                let tools = vec![
                    ToolType::CodeEditorTool,
                    ToolType::AttemptCompletion,
                    ToolType::TerminalCommand,
                ];

                let selector = Selector::new(
                    1.0,
                    false,
                    1.0,
                    0.8,
                    0.0,
                    50.0,
                    0.0,
                    75.0,
                    50.0,
                    20.0,
                    5.0,
                    50.0,
                    50.0,
                    bad_actions,
                    100.0,
                    25.0,
                    50.0,
                );

                let expansions = 1;

                let mut search_tree = SearchTree::new(
                    expansions,
                    30,
                    400,
                    Some(5),
                    None,
                    Some(2),
                    Some(1),
                    repo_location.to_string_lossy().to_string(),
                    repo_name,
                    "".to_owned(),
                    problem_statement,
                    selector,
                    tools,
                    tool_box,
                    llm_broker,
                    log_directory.to_string_lossy().to_string(),
                    agent_settings,
                );

                search_tree.run_search(message_properties).await;
                Ok(())
            })
        })
        .collect();

    // Wait for all tasks to complete
    for handle in handles {
        handle.await??;
    }

    Ok(())
}
