# Agent REPL

A REPL-like CLI tool for interacting with an AI agent that can analyze and modify code repositories.

## Features

- Point the agent to any repository
- Run queries against the repository
- Watch the agent's thought process and tool usage in real-time
- Track token usage
- Monitor files opened and edited
- Provide feedback to the agent
- Stop the agent at any point

## Installation

```bash
cargo build --release
```

The binary will be available at `target/release/agent_repl`.

## Usage

```bash
# Run with a repository path
agent_repl --repo-path /path/to/repository --api-key your_api_key

# Or set these values in the REPL
agent_repl
```

## REPL Commands

- `repo <path>` - Set the repository path
- `key <api_key>` - Set the API key
- `run <query>` - Run the agent with the given query
- `stop` - Stop the agent
- `feedback <message>` - Provide feedback to the agent
- `status` - Show the current agent status
- `help` - Show the help message
- `exit` - Exit the REPL

## Example

```
$ agent_repl
Welcome to the Agent REPL!
Type 'help' for a list of commands, 'exit' to quit
agent> repo /path/to/repository
Repository path set to: /path/to/repository
agent> key your_api_key
API key set
agent> run Add error handling to the main function
Using tool: ListFiles
Thinking: I need to understand the repository structure first. Let me list the files.
Tool result: /path/to/repository/src/main.rs
/path/to/repository/src/lib.rs
/path/to/repository/Cargo.toml

Using tool: SearchFiles
Thinking: Now I need to find files that might be relevant to the query.
Tool result: /path/to/repository/src/main.rs:10: fn main() {
/path/to/repository/src/main.rs:11:     let result = do_something();
/path/to/repository/src/main.rs:12:     println!("Result: {}", result);
/path/to/repository/src/main.rs:13: }

Token usage: 300 tokens (total: 300)
...
```

## How It Works

The agent follows these steps:

1. Analyzes the repository structure
2. Identifies relevant files
3. Reads and understands the code
4. Makes necessary changes
5. Verifies the changes work as expected
6. Provides a summary of what was done

This process mirrors the agent loop in the sidecar codebase, where the agent repeatedly:
- Selects the next tool to use
- Executes the tool
- Processes the result
- Continues until the task is complete

## Implementation Details

This tool is a simplified version of the agent loop in the sidecar codebase. It simulates the key components:

- `SessionService` - Manages the agent loop
- `Session` - Maintains the state of the conversation
- `ToolUseAgent` - Determines which tool to use next
- `ActionNode` - Tracks the tools used and their results

The actual implementation would use an LLM to decide which tool to use next, but this simulation follows a predefined sequence of tools for demonstration purposes.