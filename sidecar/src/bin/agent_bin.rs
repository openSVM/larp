use std::{path::PathBuf, sync::Arc};

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

/// Creates and returns the session storage path for a given session ID.
/// 
/// This function ensures that the session storage directory exists and creates it if necessary.
/// The path is constructed by joining the index directory with "session" and the provided session ID.
///
/// # Arguments
/// * `config` - Application configuration containing the index directory
/// * `session_id` - Unique identifier for the session
///
/// # Returns
/// A string containing the absolute path to the session storage directory
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

/// Define the command-line arguments
#[derive(Parser, Debug)]
#[command(
    author = "skcd",
    version = "1.0",
    about = "Agent binary sidecar runner"
)]
struct CliArgs {
    /// Git directory name
    #[arg(long)]
    timeout: usize,

    /// Endpoint URL
    #[arg(long)]
    editor_url: String,

    /// Timeout in seconds
    #[arg(long)]
    input: PathBuf,

    /// Anthropic api key
    #[arg(long, default_value = None)]
    anthropic_api_key: String,

    /// OPen Router api key
    #[arg(long, default_value = None)]
    openrouter_api_key: Option<String>,

    /// The run id for the current run
    #[arg(long)]
    run_id: String,

    #[arg(long)]
    repo_name: String,

    /// Directory to dump all the logs into
    #[arg(long)]
    log_directory: String,

    /// Use json mode strictly
    #[arg(long, default_value = "true")]
    json_mode: bool,

    /// Use midwit mode (aka sonnet3.5 with tool)
    #[arg(long, default_value = "true")]
    midwit_mode: bool,

    /// Run in single trajectory but a lot of them
    #[arg(long, default_value = None)]
    single_traj_search: Option<usize>,

    /// Maximum depth for the search tree
    #[arg(long, default_value = "30")]
    max_depth: u32,

    /// Model name override
    #[arg(long)]
    model_name: Option<String>,
}

/// Represents a benchmark instance for software engineering tasks.
/// Contains all necessary information about a repository, problem statement,
/// and test configurations.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SWEbenchInstance {
    /// Repository URL or identifier
    repo: String,
    /// Unique identifier for this benchmark instance
    instance_id: String,
    /// Base commit hash to start from
    base_commit: String,
    /// Patch to be applied
    patch: String,
    /// Test patch to be applied
    test_patch: String,
    /// Description of the problem to be solved
    problem_statement: String,
    /// Additional hints for solving the problem
    hints_text: String,
    /// Creation timestamp
    created_at: String,
    /// Version identifier
    version: String,
    /// Tests that should transition from failing to passing
    #[serde(rename = "FAIL_TO_PASS")]
    fail_to_pass: String,
    /// Tests that should remain passing
    #[serde(rename = "PASS_TO_PASS")]
    pass_to_pass: String,
    /// Commit hash for environment setup
    environment_setup_commit: String,
}

/// Input structure for the agent binary that combines git directory information
/// with a benchmark instance.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct InputParts {
    /// The directory name containing the git repository
    git_drname: String,
    /// The benchmark instance to be processed
    instance: SWEbenchInstance,
}
/// Main entry point for the agent binary.
/// 
/// Sets up and runs the agent with the following workflow:
/// 1. Parses command line arguments
/// 2. Configures application and logging
/// 3. Sets up LLM provider and messaging
/// 4. Processes input and executes the agent
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("agent::start");
    let args = CliArgs::parse();
    eprintln!("run_id::{}", &args.run_id);

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

    let llm_model = if let Some(model_name) = args.model_name {
        LLMType::Custom(model_name)
    } else {
        LLMType::ClaudeSonnet
    };

    let llm_provider = LLMProperties::new(
        llm_model,
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

    let input_path = args.input;
    let input_content = tokio::fs::read(input_path).await.expect("path content");
    let input_parts: InputParts =
        serde_json::from_slice(&input_content).expect("Parse the serde json");

    let cloned_session_id = args.run_id.to_string();
    let user_message = input_parts.instance.problem_statement.clone();
    let cloned_working_directory = input_parts.git_drname.to_owned();
    let tool_box = application.tool_box.clone();
    let llm_broker = application.llm_broker.clone();

    let aide_rules = Some(format!(
        r#"You are solving a github issue present in {}
FOLLOW these steps to resolve the issue:
1. As a first step, it might be a good idea to explore the repo to familiarize yourself with its structure.
2. Edit the sourcecode of the repo to resolve the issue
3. Think about edgecases and make sure your fix handles them as well

Your thinking should be thorough and so it's fine if it's very long."#,
        args.repo_name,
    ));

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
            aide_rules,
            false,
            false,
            false,
            Some(args.log_directory.clone()),
            Some(args.repo_name.clone()),
            message_properties,
            false, // not in devtools context
        )
        .await;
    println!("agent::tool_use::end");
    Ok(())
}