use std::sync::Arc;

/// This contains the binary responsible for running the agents as a farm
/// Dead simple where the inputs are the input to the git repository containing the input
/// and the problem statement, keeping it super simple and limited
use clap::Parser;
use llm_client::{
    clients::types::LLMType,
    provider::{AnthropicAPIKey, LLMProvider, LLMProviderAPIKeys},
};
use sidecar::{
    agentic::symbol::{
        events::{input::SymbolEventRequestId, message_event::SymbolEventMessageProperties},
        identifier::LLMProperties,
    },
    application::{application::Application, config::configuration::Configuration},
    repo::types::RepoRef,
    user_context::types::UserContext,
};

pub async fn check_session_storage_path(config: Arc<Configuration>, session_id: String) -> String {
    let mut session_path = config.index_dir.clone();
    session_path = session_path.join("session");
    // check if the plan_storage_path_exists
    if tokio::fs::metadata(&session_path).await.is_err() {
        tokio::fs::create_dir(&session_path)
            .await
            .expect("directory creation to not fail");
    }
    session_path = session_path.join(session_id);
    session_path
        .to_str()
        .expect("path conversion to work on all platforms")
        .to_owned()
}

#[derive(Parser, Debug)]
#[command(
    author = "skcd",
    version = "0.1",
    about = "Agent bin to point to a repo and run it (assumes an editor is running somewhere)"
)]
struct AgentParameters {
    #[arg(long)]
    timeout: usize,

    #[arg(long)]
    editor_url: String,

    #[arg(long)]
    anthropic_api_key: String,

    #[arg(long)]
    run_id: String,

    #[arg(long)]
    log_directory: String,

    #[arg(long)]
    problem_statement: String,

    #[arg(long)]
    working_directory: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("agent::start");
    let args = AgentParameters::parse();

    let mut configuration = Configuration::default();
    // we apply the edits directly over here
    configuration.apply_directly = true;

    // setup the application
    Application::install_logging(&configuration);
    Application::setup_scratch_pad(&configuration).await;

    let application = Application::initialize(configuration)
        .await
        .expect("application setup should work");
    let exchange_id = "0".to_owned();

    let llm_provider = LLMProperties::new(
        LLMType::ClaudeSonnet,
        LLMProvider::Anthropic,
        LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new(args.anthropic_api_key.to_owned())),
    );
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let message_properties = SymbolEventMessageProperties::new(
        SymbolEventRequestId::new("0".to_owned(), args.run_id.to_owned()),
        sender.clone(),
        args.editor_url.clone(),
        cancellation_token.clone(),
        llm_provider,
    );

    let session_storage_path =
        check_session_storage_path(application.config.clone(), args.run_id.clone()).await;

    let session_service = application.session_service.clone();

    let cloned_session_id = args.run_id.to_string();
    let user_message = args.problem_statement.clone();
    let cloned_working_directory = args.working_directory.clone();
    let tool_box = application.tool_box.clone();
    let llm_broker = application.llm_broker.clone();

    // wait for the agent to finish over here while busy looping
    println!("agent::tool_use::start");
    let _ = session_service
        .tool_use_agentic(
            cloned_session_id,
            session_storage_path,
            user_message,
            exchange_id,
            vec![],
            vec![],
            "bash".to_owned(),
            vec![],
            RepoRef::local(&cloned_working_directory).expect("repo_ref to work"),
            cloned_working_directory,
            tool_box,
            llm_broker,
            UserContext::default(),
            false,
            message_properties,
        )
        .await;
    println!("agent::tool_use::end");
    Ok(())
}
