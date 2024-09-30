use futures::future::try_join_all;
use std::io::{self, Write};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use tokio::fs::File;
use tokio::io::AsyncReadExt;

use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    config::LLMBrokerConfiguration,
    provider::{AnthropicAPIKey, LLMProvider, LLMProviderAPIKeys, OpenAIProvider},
};
use sidecar::{
    agentic::{
        symbol::{
            events::{input::SymbolEventRequestId, message_event::SymbolEventMessageProperties},
            identifier::LLMProperties,
            manager::SymbolManager,
            tool_box::ToolBox,
        },
        tool::{
            broker::{ToolBroker, ToolBrokerConfiguration},
            code_edit::models::broker::CodeEditBroker,
            plan::service::PlanService,
        },
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

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "plan_executor", about = "A simple plan execution tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute the next step in the plan
    Next,
}

#[tokio::main]
async fn main() {
    let request_id = uuid::Uuid::new_v4();
    let request_id_str = request_id.to_string();
    let parea_url = format!(
        r#"https://app.parea.ai/logs?colViz=%7B%220%22%3Afalse%2C%221%22%3Afalse%2C%222%22%3Afalse%2C%223%22%3Afalse%2C%22error%22%3Afalse%2C%22deployment_id%22%3Afalse%2C%22feedback_score%22%3Afalse%2C%22time_to_first_token%22%3Afalse%2C%22scores%22%3Afalse%2C%22start_timestamp%22%3Afalse%2C%22user%22%3Afalse%2C%22session_id%22%3Afalse%2C%22target%22%3Afalse%2C%22experiment_uuid%22%3Afalse%2C%22dataset_references%22%3Afalse%2C%22in_dataset%22%3Afalse%2C%22event_type%22%3Afalse%2C%22request_type%22%3Afalse%2C%22evaluation_metric_names%22%3Afalse%2C%22request%22%3Afalse%2C%22calling_node%22%3Afalse%2C%22edges%22%3Afalse%2C%22metadata_evaluation_metric_names%22%3Afalse%2C%22metadata_event_type%22%3Afalse%2C%22metadata_0%22%3Afalse%2C%22metadata_calling_node%22%3Afalse%2C%22metadata_edges%22%3Afalse%2C%22metadata_root_id%22%3Afalse%7D&filter=%7B%22filter_field%22%3A%22meta_data%22%2C%22filter_operator%22%3A%22equals%22%2C%22filter_key%22%3A%22root_id%22%2C%22filter_value%22%3A%22{request_id_str}%22%7D&page=1&page_size=50&time_filter=1m"#
    );
    println!("===========================================\nRequest ID: {}\nParea AI: {}\n===========================================", request_id.to_string(), parea_url);
    let editor_url = "http://localhost:42428".to_owned();
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
        // for our testing workflow we want to apply the edits directly
        ToolBrokerConfiguration::new(None, true),
        LLMProperties::new(
            LLMType::Gpt4O,
            LLMProvider::OpenAI,
            LLMProviderAPIKeys::OpenAI(OpenAIProvider::new(
                "sk-proj-BLaSMsWvoO6FyNwo9syqT3BlbkFJo3yqCyKAxWXLm4AvePtt".to_owned(),
            )),
        ),
    ));

    let (sender, mut _receiver) = tokio::sync::mpsc::unbounded_channel();

    let event_properties = SymbolEventMessageProperties::new(
        SymbolEventRequestId::new(request_id_str.to_owned(), request_id_str.to_owned()),
        sender.clone(),
        editor_url.to_owned(),
        tokio_util::sync::CancellationToken::new(),
    );

    let _symbol_manager = SymbolManager::new(
        tool_broker.clone(),
        symbol_broker.clone(),
        editor_parsing.clone(),
        anthropic_llm_properties.clone(),
    );

    let tool_box = Arc::new(ToolBox::new(
        tool_broker.clone(),
        symbol_broker.clone(),
        editor_parsing.clone(),
    ));

    let user_query =
        "I want you to finish implementing the create file tool, follow what we are doing in open_file. the endpoint we want to hit is `create_file`"
            .to_string();

    let _initial_context = String::from("");

    let context_files = vec![
        "/Users/skcd/scratch/sidecar/sidecar/src/agentic/tool/input.rs",
        "/Users/skcd/scratch/sidecar/sidecar/src/agentic/tool/output.rs",
        "/Users/skcd/scratch/sidecar/sidecar/src/agentic/tool/type.rs",
        "/Users/skcd/scratch/sidecar/sidecar/src/agentic/tool/errors.rs",
        "/Users/skcd/scratch/sidecar/sidecar/src/agentic/tool/lsp/open_file.rs",
        "/Users/skcd/scratch/sidecar/sidecar/src/agentic/tool/lsp/create_file.rs",
        "/Users/skcd/scratch/sidecar/sidecar/src/agentic/tool/broker.rs",
    ];

    let file_futures: Vec<_> = context_files
        .into_iter()
        .map(|path| read_file(PathBuf::from(path)))
        .collect();

    let file_contents = try_join_all(file_futures).await.unwrap();

    let user_context = UserContext::new(vec![], file_contents, None, vec![]); // this is big, should be passed using references

    let _ui_sender = event_properties.ui_sender();

    let plan_service = PlanService::new(
        tool_broker,
        tool_box.clone(),
        anthropic_llm_properties,
    );

    let path = "/Users/skcd/scratch/sidecar/sidecar/src/bin/plan.json";

    // when adding variables to the JSON, just use file_content_map (copy what you see in global context)

    let plan = if Path::new(path).exists() {
        plan_service.load_plan(path).unwrap()
    } else {
        plan_service
            .create_plan(user_query, user_context, event_properties.clone())
            .await
            .expect("Failed to create new plan")
    };

    let _ = plan_service.save_plan(&plan, path);

    println!("Welcome to Agentic Planning.");
    println!();
    println!(
        "Your plan has {} steps. We are at checkpoint {}.",
        &plan.steps().len(),
        &plan.checkpoint()
    );
    println!();

    loop {
        let mut plan = plan_service.load_plan(path).unwrap();
        let steps = plan.steps();
        let checkpoint = plan.checkpoint();
        let step_to_execute = steps.get(checkpoint).unwrap();
        let context = plan_service.prepare_context(steps, checkpoint).await;

        println!("Next step: {}", step_to_execute.title());

        println!("[1] Execute");
        println!("[2] Exit");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        match input.trim() {
            "1" | "next" => {
                // using file as store for Plan

                let _ = match plan_service.execute_step(step_to_execute, context, event_properties.clone()).await {
                    Ok(_) => {
                        println!("Checkpoint {} complete", plan.checkpoint());
                        plan.increment_checkpoint();

                        // save!
                        if let Err(e) = plan_service.save_plan(&plan, path) {
                            eprintln!("Error saving plan: {}", e)
                        }
                    }
                    Err(e) => println!("Error executing step: {}", e),
                };
            }
            "2" | "exit" => break,
            _ => println!("Invalid command. Please try again."),
        }

        println!(); // Add a blank line for readability
    }

    println!("Exiting program. Check plan's checkpoint value in JSON before next run");
}

async fn read_file(path: PathBuf) -> Result<FileContentValue, std::io::Error> {
    let mut file = File::open(&path).await?;
    let mut content = String::new();
    file.read_to_string(&mut content).await?;
    Ok(FileContentValue::new(
        path.to_string_lossy().into_owned(),
        content,
        "rs".to_owned(),
    ))
}
