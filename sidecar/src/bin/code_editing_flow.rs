use std::{path::PathBuf, sync::Arc};

use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    config::LLMBrokerConfiguration,
    provider::{AnthropicAPIKey, LLMProvider, LLMProviderAPIKeys, OpenAIProvider},
};
use sidecar::{
    agentic::{
        symbol::{
            events::input::SymbolInputEvent, identifier::LLMProperties, manager::SymbolManager,
        },
        tool::{broker::ToolBroker, code_edit::models::broker::CodeEditBroker},
    },
    chunking::{editor_parsing::EditorParsing, languages::TSLanguageParsing},
    inline_completion::symbols_tracker::SymbolTrackerInline,
    user_context::types::{FileContentValue, UserContext},
};

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
    let editor_url = "http://localhost:42450".to_owned();
    let anthropic_api_keys = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("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".to_owned()));
    let anthropic_llm_properties = LLMProperties::new(
        LLMType::ClaudeSonnet,
        LLMProvider::Anthropic,
        anthropic_api_keys.clone(),
    );
    let editor_parsing = Arc::new(EditorParsing::default());
    let symbol_broker = Arc::new(SymbolTrackerInline::new(editor_parsing.clone()));
    let tool_broker = Arc::new(ToolBroker::new(
        Arc::new(
            LLMBroker::new(LLMBrokerConfiguration::new(default_index_dir()))
                .await
                .expect("to initialize properly"),
        ),
        Arc::new(CodeEditBroker::new()),
        symbol_broker.clone(),
        Arc::new(TSLanguageParsing::init()),
        None,
        LLMProperties::new(
            LLMType::Gpt4O,
            LLMProvider::OpenAI,
            LLMProviderAPIKeys::OpenAI(OpenAIProvider::new(
                "sk-proj-BLaSMsWvoO6FyNwo9syqT3BlbkFJo3yqCyKAxWXLm4AvePtt".to_owned(),
            )),
        ), // LLMProperties::new(
           //     LLMType::GeminiPro,
           //     LLMProvider::GoogleAIStudio,
           //     LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new(
           //         "AIzaSyCMkKfNkmjF8rTOWMg53NiYmz0Zv6xbfsE".to_owned(),
           //     )),
           // ),
    ));

    // let file_path = "/Users/skcd/test_repo/sidecar/llm_client/src/provider.rs";

    let zi_test_repo_path = PathBuf::from("/Users/zi/codestory/testing/sidecar");
    // let skc_test_repo_path = PathBuf::from("/Users/skcd/test_repo/sidecar");

    let file_path = "sidecar/src/agentic/symbol/ui_event.rs";

    let file_path = zi_test_repo_path
        .join(file_path)
        .to_str()
        .unwrap()
        .to_owned();

    // let file_path =
    //     "/Users/skcd/test_repo/sidecar/sidecar/src/agentic/symbol/ui_event.rs".to_owned();

    // read the file contents
    let file_contents = String::from_utf8(
        tokio::fs::read(file_path.to_owned())
            .await
            .expect("to work"),
    )
    .expect("to work");

    let user_context = UserContext::new(
        vec![],
        vec![FileContentValue::new(
            file_path.to_owned(),
            file_contents,
            "rust".to_owned(),
        )],
        None,
        vec![],
    );

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    let symbol_manager = SymbolManager::new(
        tool_broker.clone(),
        symbol_broker.clone(),
        editor_parsing,
        editor_url.to_owned(),
        sender,
        anthropic_llm_properties.clone(),
        user_context.clone(),
        request_id.to_string(),
    );

    // let problem_statement =
    //     "can you add a new method to CodeStoryLLMTypes for setting the llm type?".to_owned();

    // let problem_statement =
    //     "can you add another provider for grok for me we just need an api_key?".to_owned();
    // let problem_statement = "Add comments to RequestEvents".to_owned();
    let problem_statement = "Implement a new SymbolEventSubStep called Document that documents symbols, implement it similar to the Edit one".to_owned();
    let initial_request = SymbolInputEvent::new(
        user_context,
        LLMType::ClaudeSonnet,
        LLMProvider::Anthropic,
        anthropic_api_keys,
        problem_statement,
        request_id.to_string(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );

    let mut initial_request_task = Box::pin(symbol_manager.initial_request(initial_request));

    loop {
        tokio::select! {
            event = receiver.recv() => {
                if let Some(_event) = event {
                    // info!("event: {:?}", event);
                } else {
                    break; // Receiver closed, exit the loop
                }
            }
            result = &mut initial_request_task => {
                match result {
                    Ok(_) => {
                        // The task completed successfully
                        // Handle the result if needed
                    }
                    Err(e) => {
                        // An error occurred while running the task
                        eprintln!("Error in initial_request_task: {}", e);
                        // Handle the error appropriately (e.g., log, retry, or exit)
                    }
                }
            }
        }
    }
}
