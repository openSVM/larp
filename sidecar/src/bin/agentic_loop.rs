use std::env;
use std::process::Command;
use std::{path::PathBuf, sync::Arc};

use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    config::LLMBrokerConfiguration,
    provider::{
        AnthropicAPIKey, FireworksAPIKey, GoogleAIStudioKey, LLMProvider, LLMProviderAPIKeys,
        OpenAIProvider,
    },
};
use sidecar::{
    agentic::{
        symbol::{
            events::{
                input::{SymbolEventRequestId, SymbolInputEvent},
                message_event::SymbolEventMessageProperties,
            },
            identifier::LLMProperties,
            manager::SymbolManager,
        },
        tool::{
            broker::{ToolBroker, ToolBrokerConfiguration},
            code_edit::models::broker::CodeEditBroker,
        },
    },
    chunking::{editor_parsing::EditorParsing, languages::TSLanguageParsing},
    inline_completion::symbols_tracker::SymbolTrackerInline,
    user_context::types::UserContext,
};

use clap::{Parser, Subcommand};

fn default_index_dir() -> PathBuf {
    match directories::ProjectDirs::from("ai", "codestory", "sidecar") {
        Some(dirs) => dirs.data_dir().to_owned(),
        None => "codestory_sidecar".into(),
    }
}

#[tokio::main]
async fn main() {
    let request_id = uuid::Uuid::new_v4();
    let request_id_str = request_id.to_string();
    let parea_url = format!(
        r#"https://app.parea.ai/logs?colViz=%7B%220%22%3Afalse%2C%221%22%3Afalse%2C%222%22%3Afalse%2C%223%22%3Afalse%2C%22error%22%3Afalse%2C%22deployment_id%22%3Afalse%2C%22feedback_score%22%3Afalse%2C%22time_to_first_token%22%3Afalse%2C%22scores%22%3Afalse%2C%22start_timestamp%22%3Afalse%2C%22user%22%3Afalse%2C%22session_id%22%3Afalse%2C%22target%22%3Afalse%2C%22experiment_uuid%22%3Afalse%2C%22dataset_references%22%3Afalse%2C%22in_dataset%22%3Afalse%2C%22event_type%22%3Afalse%2C%22request_type%22%3Afalse%2C%22evaluation_metric_names%22%3Afalse%2C%22request%22%3Afalse%2C%22calling_node%22%3Afalse%2C%22edges%22%3Afalse%2C%22metadata_evaluation_metric_names%22%3Afalse%2C%22metadata_event_type%22%3Afalse%2C%22metadata_0%22%3Afalse%2C%22metadata_calling_node%22%3Afalse%2C%22metadata_edges%22%3Afalse%2C%22metadata_root_id%22%3Afalse%7D&filter=%7B%22filter_field%22%3A%22meta_data%22%2C%22filter_operator%22%3A%22equals%22%2C%22filter_key%22%3A%22root_id%22%2C%22filter_value%22%3A%22{request_id_str}%22%7D&page=1&page_size=50&time_filter=1m"#
    );
    println!("===========================================\nRequest ID: {}\nParea AI: {}\n===========================================", request_id.to_string(), parea_url);
    let editor_url = "http://localhost:42425".to_owned();
    let anthropic_api_keys = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("".to_owned()));
    let anthropic_llm_properties = LLMProperties::new(
        LLMType::ClaudeSonnet,
        LLMProvider::Anthropic,
        anthropic_api_keys.clone(),
    );
    let _llama_70b_properties = LLMProperties::new(
        LLMType::Llama3_1_70bInstruct,
        LLMProvider::FireworksAI,
        LLMProviderAPIKeys::FireworksAI(FireworksAPIKey::new(
            "s8Y7yIXdL0lMeHHgvbZXS77oGtBAHAsfsLviL2AKnzuGpg1n".to_owned(),
        )),
    );
    let _google_ai_studio_api_keys =
        LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new("".to_owned()));
    let editor_parsing = Arc::new(EditorParsing::default());
    let symbol_broker = Arc::new(SymbolTrackerInline::new(editor_parsing.clone()));
    let llm_broker = LLMBroker::new(LLMBrokerConfiguration::new(default_index_dir()))
        .await
        .expect("to initialize properly");

    let llm_broker_clone = LLMBroker::new(LLMBrokerConfiguration::new(default_index_dir()))
        .await
        .expect("to initialize properly");

    let tool_broker = Arc::new(ToolBroker::new(
        Arc::new(llm_broker),
        Arc::new(CodeEditBroker::new()),
        symbol_broker.clone(),
        Arc::new(TSLanguageParsing::init()),
        // for our testing workflow we want to apply the edits directly
        ToolBrokerConfiguration::new(None, true),
        LLMProperties::new(
            LLMType::Gpt4O,
            LLMProvider::OpenAI,
            LLMProviderAPIKeys::OpenAI(OpenAIProvider::new("".to_owned())),
        ), // LLMProperties::new(
           //     LLMType::GeminiPro,
           //     LLMProvider::GoogleAIStudio,
           //     LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new(
           //         "".to_owned(),
           //     )),
           // ),
    ));

    let user_context = UserContext::new(vec![], vec![], None, vec![]);

    let (sender, mut _receiver) = tokio::sync::mpsc::unbounded_channel();

    // fill this
    let access_token = String::from("");

    let _event_properties = SymbolEventMessageProperties::new(
        SymbolEventRequestId::new("".to_owned(), "".to_owned()),
        sender.clone(),
        editor_url.to_owned(),
        tokio_util::sync::CancellationToken::new(),
        access_token,
    );

    let _symbol_manager = SymbolManager::new(
        tool_broker.clone(),
        symbol_broker.clone(),
        editor_parsing,
        anthropic_llm_properties.clone(),
    );

    struct TerminalCommandGenerator {
        pub model: LLMType,
        pub provider: LLMProvider,
        pub api_keys: LLMProviderAPIKeys,
        pub _root_directory: String,
        pub root_request_id: String,
        pub client: Arc<LLMBroker>,
    }

    // ANTHROPIC_API_KEY
    let api_key = env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| "".to_string());

    match env::var("ANTHROPIC_API_KEY") {
        Ok(key) => println!("API key: {}", key),
        Err(_) => println!("API key not found"),
    }

    let terminal_command_generator = TerminalCommandGenerator {
        model: LLMType::ClaudeSonnet,
        provider: LLMProvider::Anthropic,
        api_keys: LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new(api_key)),
        _root_directory: "".to_owned(),
        root_request_id: "".to_owned(),
        client: Arc::new(llm_broker_clone),
    };

    println!("Interactive CLI Tool (type 'exit' to quit)");

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let input = input.trim();

        if input == "exit" {
            println!("Goodbye!");
            break;
        }

        // Process the input
        process_input(input);
    }
}

use std::io::{self, Write};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Clone)]
enum Commands {
    Request {
        #[arg(short, long)]
        query: String,
    },
}

#[derive(Debug)]
enum SystemState {
    Thinking,
    UsingTool1,
    UsingTool2,
    UsingTool3,
}

fn process_input(query: &str) {
    println!("Received request: {}", query);

    // Enter thinking state
    let state = SystemState::Thinking;
    println!("System state: {:?}", state);

    // Simulate thinking and tool selection
    let selected_tool = rand::random::<u8>() % 1;

    // Transition to appropriate tool state
    let state = match selected_tool {
        0 => SystemState::UsingTool1,
        1 => SystemState::UsingTool2,
        _ => SystemState::UsingTool3,
    };

    println!("Selected tool state: {:?}", state);

    match state {
        SystemState::UsingTool1 => {
            println!("Using Tool 1 (terminal) to process request...");

            let input = "ls";
            let output = execute_terminal_command(input).unwrap();

            println!("Command output: {}", output);
            // Tool 1 specific logic would go here
        }
        SystemState::UsingTool2 => {
            println!("Using Tool 2 to process request...");
            // Tool 2 specific logic would go here
        }
        SystemState::UsingTool3 => {
            println!("Using Tool 3 to process request...");
            // Tool 3 specific logic would go here
        }
        _ => {}
    }
}

// Simple wrapper that returns Result directly for better error handling
fn execute_terminal_command(command: &str) -> std::io::Result<String> {
    let output = Command::new(command).output()?;

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
