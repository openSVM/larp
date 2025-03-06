use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use llm_client::clients::types::LLMType;

mod agent;
mod models;
mod tools;

use agent::AgentState;
use models::{AgentAction, AgentResponse, ToolType, TokenUsage};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the repository to analyze
    #[arg(short, long)]
    repo_path: Option<PathBuf>,

    /// API key for the LLM service
    #[arg(short, long)]
    api_key: Option<String>,
    
    /// API key for OpenRouter
    #[arg(long)]
    openrouter_api_key: Option<String>,
    
    /// API key for Anthropic
    #[arg(long)]
    anthropic_api_key: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    env_logger::init();
    
    // Initialize the agent state
    let agent_state = Arc::new(Mutex::new(AgentState::new()));
    
    // Set the repository path if provided
    if let Some(repo_path) = args.repo_path {
        let repo_path = repo_path.canonicalize().context("Failed to canonicalize repository path")?;
        agent_state.lock().unwrap().set_repo_path(repo_path);
        println!("{} {}", "Repository path set to:".green(), agent_state.lock().unwrap().repo_path().unwrap().display());
    }
    
    // Set the API key if provided from args or environment
    let api_key = args.api_key.or_else(|| std::env::var("LLM_API_KEY").ok());
    if let Some(api_key) = api_key {
        agent_state.lock().unwrap().set_api_key(api_key);
        println!("{}", "API key set".green());
    }
    
    // Set the OpenRouter API key if provided from args or environment
    let openrouter_api_key = args.openrouter_api_key.or_else(|| std::env::var("OPENROUTER_API_KEY").ok());
    if let Some(api_key) = openrouter_api_key {
        agent_state.lock().unwrap().set_openrouter_api_key(api_key);
        println!("{}", "OpenRouter API key set".green());
    }
    
    // Set the Anthropic API key if provided from args or environment
    let anthropic_api_key = args.anthropic_api_key.or_else(|| std::env::var("ANTHROPIC_API_KEY").ok());
    if let Some(api_key) = anthropic_api_key {
        agent_state.lock().unwrap().set_anthropic_api_key(api_key);
        println!("{}", "Anthropic API key set".green());
    }
    
    // Start the REPL
    run_repl(agent_state).await?;
    
    Ok(())
}

async fn run_repl(agent_state: Arc<Mutex<AgentState>>) -> Result<()> {
    let mut rl = DefaultEditor::new()?;
    
    println!("{}", "Welcome to the Agent REPL!".bold().green());
    println!("Type 'help' for a list of commands, 'exit' to quit");
    
    loop {
        let readline = rl.readline("agent> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;
                
                if line.trim().is_empty() {
                    continue;
                }
                
                match line.trim() {
                    "exit" | "quit" => {
                        println!("Goodbye!");
                        break;
                    },
                    "help" => {
                        print_help();
                    },
                    "status" => {
                        print_status(&agent_state);
                    },
                    cmd if cmd.starts_with("repo ") => {
                        let path = cmd.trim_start_matches("repo ").trim();
                        let path = PathBuf::from(path);
                        let path = path.canonicalize().context("Failed to canonicalize repository path")?;
                        agent_state.lock().unwrap().set_repo_path(path);
                        println!("{} {}", "Repository path set to:".green(), agent_state.lock().unwrap().repo_path().unwrap().display());
                    },
                    cmd if cmd.starts_with("run ") => {
                        let query = cmd.trim_start_matches("run ").trim();
                        if agent_state.lock().unwrap().repo_path().is_none() {
                            println!("{}", "Repository path not set. Use 'repo <path>' to set it.".red());
                            continue;
                        }
                        
                        // Check if the appropriate API key is set based on the model type
                        let model_type = agent_state.lock().unwrap().llm_type().clone();
                        let api_key_missing = match model_type {
                            LLMType::ClaudeSonnet | LLMType::ClaudeHaiku | LLMType::ClaudeOpus => {
                                agent_state.lock().unwrap().anthropic_api_key().is_none()
                            },
                            LLMType::Custom(ref name) if name.contains("anthropic") => {
                                agent_state.lock().unwrap().anthropic_api_key().is_none()
                            },
                            LLMType::Custom(ref name) if name.contains("openrouter") => {
                                agent_state.lock().unwrap().openrouter_api_key().is_none()
                            },
                            _ => agent_state.lock().unwrap().api_key().is_none(),
                        };
                        
                        if api_key_missing {
                            println!("{}", "API key not set for the selected model. Use 'key <api_key>', 'openrouter_key <api_key>', or 'anthropic_key <api_key>' to set it.".red());
                            continue;
                        }
                        
                        run_agent(agent_state.clone(), query.to_string()).await?;
                    },
                    cmd if cmd.starts_with("key ") => {
                        let api_key = cmd.trim_start_matches("key ").trim();
                        agent_state.lock().unwrap().set_api_key(api_key.to_string());
                        println!("{}", "API key set".green());
                    },
                    cmd if cmd.starts_with("openrouter_key ") => {
                        let api_key = cmd.trim_start_matches("openrouter_key ").trim();
                        agent_state.lock().unwrap().set_openrouter_api_key(api_key.to_string());
                        println!("{}", "OpenRouter API key set".green());
                    },
                    cmd if cmd.starts_with("anthropic_key ") => {
                        let api_key = cmd.trim_start_matches("anthropic_key ").trim();
                        agent_state.lock().unwrap().set_anthropic_api_key(api_key.to_string());
                        println!("{}", "Anthropic API key set".green());
                    },
                    cmd if cmd.starts_with("timeout ") => {
                        let timeout_str = cmd.trim_start_matches("timeout ").trim();
                        match timeout_str.parse::<u64>() {
                            Ok(seconds) => {
                                let duration = Duration::from_secs(seconds);
                                agent_state.lock().unwrap().set_timeout_duration(duration);
                                println!("{} {}s", "Timeout set to:".green(), seconds);
                            },
                            Err(_) => {
                                println!("{}", "Invalid timeout value. Please provide a number in seconds.".red());
                            }
                        }
                    },
                    cmd if cmd.starts_with("model ") => {
                        let model_name = cmd.trim_start_matches("model ").trim();
                        let llm_type = match model_name.to_lowercase().as_str() {
                            "claude-sonnet" => LLMType::ClaudeSonnet,
                            "claude-haiku" => LLMType::ClaudeHaiku,
                            "claude-opus" => LLMType::ClaudeOpus,
                            "gpt-4" => LLMType::Gpt4,
                            "gpt-4o" => LLMType::Gpt4O,
                            "gemini-pro" => LLMType::GeminiPro,
                            _ => LLMType::Custom(model_name.to_string()),
                        };
                        agent_state.lock().unwrap().set_llm_type(llm_type);
                        println!("{} {}", "LLM model set to:".green(), model_name);
                    },
                    cmd if cmd.starts_with("stop") => {
                        agent_state.lock().unwrap().stop_agent();
                        println!("{}", "Agent stopped".yellow());
                    },
                    cmd if cmd.starts_with("feedback ") => {
                        let feedback = cmd.trim_start_matches("feedback ").trim();
                        agent_state.lock().unwrap().add_feedback(feedback.to_string());
                        println!("{}", "Feedback added".green());
                    },
                    _ => {
                        println!("{}", "Unknown command. Type 'help' for a list of commands.".red());
                    }
                }
            },
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            },
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            },
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    
    Ok(())
}

fn print_help() {
    println!("{}", "Available commands:".bold());
    println!("  {} - Set the repository path", "repo <path>".cyan());
    println!("  {} - Set the API key", "key <api_key>".cyan());
    println!("  {} - Set the OpenRouter API key", "openrouter_key <api_key>".cyan());
    println!("  {} - Set the Anthropic API key", "anthropic_key <api_key>".cyan());
    println!("  {} - Set the timeout in seconds", "timeout <seconds>".cyan());
    println!("  {} - Set the LLM model", "model <model_name>".cyan());
    println!("  {} - Run the agent with the given query", "run <query>".cyan());
    println!("  {} - Stop the agent", "stop".cyan());
    println!("  {} - Provide feedback to the agent", "feedback <message>".cyan());
    println!("  {} - Show the current agent status", "status".cyan());
    println!("  {} - Show this help message", "help".cyan());
    println!("  {} - Exit the REPL", "exit".cyan());
}

fn print_status(agent_state: &Arc<Mutex<AgentState>>) {
    let state = agent_state.lock().unwrap();
    
    println!("{}", "Agent Status:".bold());
    println!("  Repository path: {}", state.repo_path().map_or("Not set".to_string(), |p| p.display().to_string()));
    println!("  API key: {}", state.api_key().map_or("Not set".to_string(), |_| "Set".to_string()));
    println!("  OpenRouter API key: {}", state.openrouter_api_key().map_or("Not set".to_string(), |_| "Set".to_string()));
    println!("  Anthropic API key: {}", state.anthropic_api_key().map_or("Not set".to_string(), |_| "Set".to_string()));
    println!("  Timeout: {:?}", state.timeout_duration());
    println!("  LLM model: {}", state.llm_type().to_string());
    println!("  Running: {}", if state.is_running() { "Yes".green() } else { "No".red() });
    println!("  Files opened: {}", state.files_opened().len());
    println!("  Files edited: {}", state.files_edited().len());
    println!("  Total tokens used: {}", state.token_usage().total());
    println!("  Current tool: {}", state.current_tool().map_or("None".to_string(), |t| format!("{:?}", t)));
}

async fn run_agent(agent_state: Arc<Mutex<AgentState>>, query: String) -> Result<()> {
    // Set the agent as running
    agent_state.lock().unwrap().start_agent(query.clone());
    
    // Create channels for communication
    let (tx, mut rx) = mpsc::channel(100);
    
    // Clone the agent state for the agent task
    let agent_state_clone = agent_state.clone();
    
    // Spawn the agent task
    let agent_handle = tokio::spawn(async move {
        agent::run_agent_loop(agent_state_clone, query, tx).await
    });
    
    // Process agent responses
    while let Some(response) = rx.recv().await {
        match response {
            AgentResponse::ToolUse { tool_type, thinking } => {
                println!("{} {}", "Using tool:".blue(), format!("{:?}", tool_type).cyan());
                println!("{} {}", "Thinking:".blue(), thinking);
                
                // Update the agent state
                let mut state = agent_state.lock().unwrap();
                state.set_current_tool(tool_type);
            },
            AgentResponse::ToolResult { result } => {
                println!("{} {}", "Tool result:".green(), result);
            },
            AgentResponse::TokenUsage { usage } => {
                let mut state = agent_state.lock().unwrap();
                let usage_clone = usage.clone(); // Clone before moving
                state.add_token_usage(usage);
                println!("{} {} tokens (total: {})", 
                    "Token usage:".yellow(), 
                    usage_clone.total(), 
                    state.token_usage().total());
            },
            AgentResponse::FileOpened { path } => {
                let mut state = agent_state.lock().unwrap();
                state.add_file_opened(path.clone());
                println!("{} {}", "File opened:".magenta(), path.display());
            },
            AgentResponse::FileEdited { path } => {
                let mut state = agent_state.lock().unwrap();
                state.add_file_edited(path.clone());
                println!("{} {}", "File edited:".magenta(), path.display());
            },
            AgentResponse::Completion { message } => {
                println!("{}", "Agent completed:".green().bold());
                println!("{}", message);
                
                // Set the agent as not running
                agent_state.lock().unwrap().stop_agent();
                break;
            },
            AgentResponse::Error { message } => {
                println!("{} {}", "Error:".red().bold(), message);
                
                // Set the agent as not running
                agent_state.lock().unwrap().stop_agent();
                break;
            },
        }
        
        // Check if the agent should be stopped
        if !agent_state.lock().unwrap().is_running() {
            println!("{}", "Agent stopped by user".yellow());
            break;
        }
    }
    
    // Wait for the agent task to complete
    let _ = agent_handle.await;
    
    Ok(())
}