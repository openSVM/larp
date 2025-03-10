use clap::{Parser, Subcommand};
use sidecar::agentic::tool::r#type::ToolType;
use sidecar::agentic::tool::broker::ToolBroker;
use std::sync::Arc;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List available tools
    Tools {
        #[clap(subcommand)]
        command: ToolsCommands,
    },
}

#[derive(Subcommand)]
enum ToolsCommands {
    /// List all available tools
    List,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Tools { command }) => match command {
            ToolsCommands::List => {
                // Create a tool broker to get the list of tools
                let tool_broker = Arc::new(ToolBroker::new());
                
                // Get the list of tools
                let tools = tool_broker.mcp_tools.clone();
                
                println!("Available tools:");
                for tool in tools.iter() {
                    match tool {
                        ToolType::McpTool(name) => println!("- {} (MCP Tool)", name),
                        _ => println!("- {:?}", tool),
                    }
                }
                
                Ok(())
            }
        },
        None => {
            println!("No command specified. Use --help for usage information.");
            Ok(())
        }
    }
}