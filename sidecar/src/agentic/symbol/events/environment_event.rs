//! Contains the environment event which might be sent externally
//! to either inform of something happening or for a request-id

use crate::agentic::symbol::{
    anchored::AnchoredSymbol,
    types::{SymbolEventRequest, SymbolEventResponse},
};

use super::{
    human::{HumanAnchorRequest, HumanMessage},
    input::SymbolEventRequestId,
    lsp::LSPSignal,
};

pub struct EditorStateChangeRequest {
    edits_made: Vec<SymbolEventResponse>,
    user_query: String,
}

impl EditorStateChangeRequest {
    pub fn new(edits_made: Vec<SymbolEventResponse>, user_query: String) -> Self {
        Self {
            edits_made,
            user_query,
        }
    }
    pub fn user_query(&self) -> &str {
        &self.user_query
    }

    pub fn consume_edits_made(self) -> Vec<SymbolEventResponse> {
        self.edits_made
    }
}

pub enum EnvironmentEventType {
    /// Represents a symbol-related event request
    Symbol(SymbolEventRequest),
    /// Indicates a change in the editor state
    EditorStateChange(EditorStateChangeRequest),
    /// Represents a Language Server Protocol signal
    LSP(LSPSignal),
    /// Represents a message from a human user
    Human(HumanMessage),
    /// A simple ping event, possibly used for keeping connections alive
    Ping,
    /// Signals that the system should shut down
    ShutDown,
}

impl EnvironmentEventType {
    pub fn is_shutdown(&self) -> bool {
        matches!(self, Self::ShutDown)
    }

    pub fn human_anchor_request(
        query: String,
        anchored_symbols: Vec<AnchoredSymbol>,
        context: Option<String>,
    ) -> Self {
        EnvironmentEventType::Human(HumanMessage::Anchor(HumanAnchorRequest::new(
            query,
            anchored_symbols,
            context,
        )))
    }

    pub fn ping() -> Self {
        EnvironmentEventType::Ping
    }
}

pub struct EnvironmentEvent {
    _request_id: SymbolEventRequestId,
    _event: EnvironmentEventType,
}

impl EnvironmentEvent {
    /// Creates a lsp signal
    pub fn lsp_signal(request_id: SymbolEventRequestId, signal: LSPSignal) -> Self {
        Self {
            _request_id: request_id,
            _event: EnvironmentEventType::LSP(signal),
        }
    }
}
