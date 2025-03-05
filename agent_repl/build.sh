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
    echo "./target/release/agent_repl --repo-path /path/to/repository --api-key your_api_key"
else
    echo "Build failed!"
fi