#!/bin/bash

# Build the agent REPL
echo "Building agent_repl..."
cargo build --release

# Check if the build was successful
if [ $? -eq 0 ]; then
    echo "Build successful!"
    echo "The binary is available at target/release/agent_repl"
    echo ""
    echo "To run the agent REPL, use:"
    echo "./target/release/agent_repl"
    echo ""
    echo "Or with arguments:"
    echo "./target/release/agent_repl --repo-path /path/to/repository --api-key your_api_key --openrouter-api-key your_openrouter_api_key --anthropic-api-key your_anthropic_api_key --timeout 300 --model claude-sonnet"
    echo ""
    echo "Available models:"
    echo "  - claude-sonnet"
    echo "  - claude-haiku"
    echo "  - claude-opus"
    echo "  - gpt-4"
    echo "  - gpt-4o"
    echo "  - gemini-pro"
    echo "  - [custom model name]"
    echo ""
    echo "API Keys:"
    echo "  - Default API Key (--api-key): Used for OpenAI models"
    echo "  - OpenRouter API Key (--openrouter-api-key): Used for models accessed through OpenRouter"
    echo "  - Anthropic API Key (--anthropic-api-key): Used for Claude models"
    echo ""
    echo "You can also set API keys using environment variables:"
    echo "  - LLM_API_KEY: Default API key"
    echo "  - OPENROUTER_API_KEY: OpenRouter API key"
    echo "  - ANTHROPIC_API_KEY: Anthropic API key"
else
    echo "Build failed!"
fi