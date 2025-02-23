# Sidecar Development Guide

This guide helps developers understand Sidecar's architecture and contribute effectively.

## Project Structure

```
sidecar/
├── src/
│   ├── agentic/      # AI agent implementation
│   ├── chunking/     # Code chunking and analysis
│   ├── git/          # Git integration
│   ├── llm/          # LLM client implementations
│   ├── repomap/      # Repository mapping
│   └── webserver/    # API endpoints
├── llm_client/       # LLM API clients
├── llm_prompts/      # Prompt templates
└── logging/          # Logging infrastructure
```

## Development Setup

1. **Prerequisites:**
   - Rust 1.73+
   - Node.js 18+
   - Git
   - VSCode with recommended extensions

2. **Environment Setup:**
   ```bash
   # Clone repository
   git clone https://github.com/yourusername/sidecar.git
   cd sidecar

   # Install Rust dependencies
   cargo build

   # Install frontend dependencies
   cd frontend
   npm install
   ```

3. **Configuration:**
   - Copy `.env.example` to `.env`
   - Set required API keys and configurations

## Core Components

### 1. AI Agent System
- Located in `src/agentic/`
- Handles code analysis and generation
- Uses LLM for intelligent operations

### 2. Repository Analysis
- Located in `src/repomap/`
- Maps code structure and relationships
- Tracks symbol definitions and usage

### 3. LLM Integration
- Located in `llm_client/`
- Supports multiple LLM providers
- Handles token management and rate limiting

### 4. Code Chunking
- Located in `src/chunking/`
- Breaks code into manageable segments
- Language-specific parsing and analysis

## Development Workflow

1. **Creating Features:**
   - Branch naming: `feature/description`
   - Follow Rust/TypeScript best practices
   - Add tests for new functionality

2. **Testing:**
   ```bash
   # Run Rust tests
   cargo test

   # Run specific test
   cargo test test_name

   # Run frontend tests
   npm test
   ```

3. **Code Style:**
   - Run formatters:
     ```bash
     cargo fmt
     cargo clippy
     ```
   - Follow project conventions

4. **Documentation:**
   - Update API docs for endpoint changes
   - Add inline documentation
   - Update README for major changes

## Debugging

1. **Backend:**
   ```rust
   // Add debug logging
   log::debug!("Processing request: {:?}", request);
   ```

2. **Frontend:**
   - Use Chrome DevTools
   - React Developer Tools
   - Network tab for API calls

## Common Tasks

1. **Adding LLM Provider:**
   - Implement provider trait in `llm_client/`
   - Add configuration in settings
   - Update provider factory

2. **New Language Support:**
   - Add parser in `chunking/`
   - Implement language-specific analysis
   - Update language configuration

3. **API Endpoints:**
   - Add route in `webserver/`
   - Implement handler
   - Update API documentation

## Performance Considerations

1. **Code Analysis:**
   - Use incremental processing
   - Implement caching where possible
   - Profile heavy operations

2. **Memory Usage:**
   - Monitor heap allocations
   - Use appropriate data structures
   - Implement cleanup for unused resources

## Error Handling

1. **Backend:**
   ```rust
   use thiserror::Error;

   #[derive(Error, Debug)]
   pub enum SidecarError {
       #[error("Invalid request: {0}")]
       InvalidRequest(String),
       #[error("LLM error: {0}")]
       LLMError(String),
   }
   ```

2. **Frontend:**
   - Implement error boundaries
   - Add user-friendly error messages
   - Log errors for debugging

## Contributing

1. **Pull Requests:**
   - Reference issue numbers
   - Include test coverage
   - Update documentation
   - Add changelog entry

2. **Code Review:**
   - Review guidelines in CONTRIBUTING.md
   - Use PR templates
   - Address all comments

## Resources

- [Architecture Documentation](./architecture.md)
- [API Documentation](./api-documentation.md)
- [Testing Guide](./testing-guide.md)