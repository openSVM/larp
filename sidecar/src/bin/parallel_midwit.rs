use clap::Parser;
use std::path::PathBuf;

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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
                let mut modified_args = CliArgs {
                    timeout,
                    repo_location,
                    repo_name,
                    anthropic_api_key,
                    openrouter_api_key,
                    problem_statement,
                };

                // Here you would put the actual processing logic that uses modified_args
                println!("Processing repo at: {:?}", modified_args.repo_location);
                // Add your actual processing logic here
            })
        })
        .collect();

    // Wait for all tasks to complete
    for handle in handles {
        handle.await?;
    }

    Ok(())
}
