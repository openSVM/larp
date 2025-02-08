//! Contains the struct for search and replace style editing

use async_trait::async_trait;
use futures::{lock::Mutex, StreamExt};
use logging::new_client;
use std::path::Path;
use std::{collections::HashMap, sync::Arc};
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc::UnboundedSender, Semaphore};

use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage},
};

use crate::{
    agentic::{
        symbol::{
            identifier::{LLMProperties, SymbolIdentifier},
            ui_event::{EditedCodeStreamingRequest, UIEventWithID},
        },
        tool::{
            errors::ToolError,
            helpers::{
                cancellation_future::run_with_cancellation, diff_recent_changes::DiffRecentChanges,
            },
            input::ToolInput,
            lsp::{diagnostics::DiagnosticWithSnippet, open_file::OpenFileRequest},
            output::ToolOutput,
            r#type::{Tool, ToolRewardScale},
            session::chat::{SessionChatMessage, SessionChatRole},
        },
    },
    chunking::text_document::{Position, Range},
};

// ... rest of the original file content ...

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_newline_preservation() {
        // Test case 1: File with existing trailing newline
        let original_code = "fn main() {\n    println!(\"Hello\");\n}\n";
        let edits = r#"```rust
<<<<<<< SEARCH
fn main() {
    println!("Hello");
}
fn test() {
    println!("New content");
}