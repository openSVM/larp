//! Contains the scratch pad agent whose job is to work alongside the developer
//! and help them accomplish a task
//! This way the agent can look at all the events and the requests which are happening
//! and take a decision based on them on what should happen next

use std::{collections::HashSet, pin::Pin, sync::Arc};

use futures::{stream, Stream, StreamExt};
use tokio::sync::{mpsc::UnboundedSender, Mutex};

use crate::agentic::symbol::{events::types::SymbolEvent, ui_event::UIEventWithID};

use super::{
    errors::SymbolError,
    events::{
        environment_event::{EditorStateChangeRequest, EnvironmentEventType},
        human::{HumanAnchorRequest, HumanMessage},
        lsp::{LSPDiagnosticError, LSPSignal},
        message_event::{SymbolEventMessage, SymbolEventMessageProperties},
    },
    tool_box::ToolBox,
    tool_properties::ToolProperties,
    types::SymbolEventRequest,
};

#[derive(Debug, Clone)]
struct ScratchPadFilesActive {
    file_content: String,
    file_path: String,
}

impl ScratchPadFilesActive {
    fn new(file_content: String, file_path: String) -> Self {
        Self {
            file_content,
            file_path,
        }
    }
}

// We should have a way to update our cache of all that has been done
// and what we are upto right now
// the ideal goal would be to rewrite the scratchpad in a good way so we are
// able to work on top of that
// a single LLM call should rewrite the sections which are present and take as input
// the lsp signal
// we also need to tell this agent what all things are possible, like: getting data from elsewhere
// looking at some other file and keeping that in its cache
// also what kind of information it should keep in:
// it can be state driven based on the user ask
// there will be files which the system has to keep in context, which can be dynamic as well
// we have to control it to not go over the 50kish limit ... cause it can grow by a lot
// but screw it, we keep it as it is
// lets keep it free-flow before we figure out the right way to go about doing this
// mega-scratchpad ftw

/// Different kind of events which can happen
/// We should move beyond symbol events tbh at this point :')

#[derive(Clone)]
pub struct ScratchPadAgent {
    storage_fs_path: String,
    message_properties: SymbolEventMessageProperties,
    tool_box: Arc<ToolBox>,
    // if the scratch-pad agent is right now focussed, then we can't react to other
    // signals and have to pay utmost attention to the current task we are workign on
    focussing: Arc<Mutex<bool>>,
    symbol_event_sender: UnboundedSender<SymbolEventMessage>,
    // This is the cache which we have to send with every request
    files_context: Arc<Mutex<Vec<ScratchPadFilesActive>>>,
    // This is the extra context which we send everytime with each request
    // this also helps with the prompt cache hits
    extra_context: Arc<Mutex<String>>,
    reaction_sender: UnboundedSender<EnvironmentEventType>,
}

impl ScratchPadAgent {
    pub async fn new(
        scratch_pad_path: String,
        message_properties: SymbolEventMessageProperties,
        tool_box: Arc<ToolBox>,
        symbol_event_sender: UnboundedSender<SymbolEventMessage>,
    ) -> Self {
        let (reaction_sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let scratch_pad_agent = Self {
            storage_fs_path: scratch_pad_path,
            message_properties,
            tool_box,
            symbol_event_sender,
            focussing: Arc::new(Mutex::new(false)),
            files_context: Arc::new(Mutex::new(vec![])),
            extra_context: Arc::new(Mutex::new("".to_owned())),
            reaction_sender: reaction_sender.clone(),
        };
        let cloned_scratch_pad_agent = scratch_pad_agent.clone();
        let mut reaction_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
        tokio::spawn(async move {
            while let Some(reaction_event) = reaction_stream.next().await {
                // presuming handled here instead of in react_to_event due to criticality
                if reaction_event.is_shutdown() {
                    break;
                }
                // react to events here
                let _ = cloned_scratch_pad_agent
                    .react_to_event(reaction_event)
                    .await;
            }
        });

        // Spawn the ping task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            loop {
                interval.tick().await;

                // if channel closed
                if let Err(_) = reaction_sender.send(EnvironmentEventType::ping()) {
                    break;
                }
            }
        });

        scratch_pad_agent
    }
}

impl ScratchPadAgent {
    /// We try to contain all the events which are coming in from the symbol
    /// which is being edited by the user, the real interface here will look like this
    pub async fn process_environment(
        self,
        mut stream: Pin<Box<dyn Stream<Item = EnvironmentEventType> + Send + Sync>>,
    ) {
        println!("scratch_pad_agent::start_processing_environment");
        while let Some(event) = stream.next().await {
            match event {
                EnvironmentEventType::LSP(lsp_signal) => {
                    // we just want to react to the lsp signal over here, so we do just that
                    let _ = self
                        .reaction_sender
                        .send(EnvironmentEventType::LSP(lsp_signal));
                }
                EnvironmentEventType::Human(message) => {
                    println!("scratch_pad_agent::human_message::({:?})", &message);
                    let _ = self.handle_human_message(message).await;
                    // whenever the human sends a request over here, encode it and try
                    // to understand how to handle it, some might require search, some
                    // might be more automagic
                }
                EnvironmentEventType::Symbol(_symbol_event) => {
                    // we know a symbol is going to be edited, what should we do about it?
                }
                EnvironmentEventType::EditorStateChange(_) => {
                    // not sure what to do about this right now, this event is used so the
                    // scratchpad can react to it, so for now do not do anything
                    // we might have to split the events later down the line
                }
                EnvironmentEventType::Ping => {
                    let _ = self.reaction_sender.send(EnvironmentEventType::Ping);
                }
                EnvironmentEventType::ShutDown => {
                    println!("scratch_pad_agent::shut_down");
                    let _ = self.reaction_sender.send(EnvironmentEventType::ShutDown);
                    break;
                }
            }
        }
    }

    async fn react_to_event(&self, event: EnvironmentEventType) {
        match event {
            EnvironmentEventType::Human(human_event) => {
                let _ = self.react_to_human_event(human_event).await;
            }
            EnvironmentEventType::EditorStateChange(editor_state_change) => {
                self.react_to_edits(editor_state_change).await;
            }
            EnvironmentEventType::LSP(lsp_signal) => {
                self.react_to_lsp_signal(lsp_signal).await;
            }
            EnvironmentEventType::Ping => {
                self.react_to_ping().await;
            }
            _ => {}
        }
    }

    async fn handle_human_message(&self, human_message: HumanMessage) -> Result<(), SymbolError> {
        match human_message {
            HumanMessage::Anchor(anchor_request) => self.human_message_anchor(anchor_request).await,
            HumanMessage::Followup(_followup_request) => Ok(()),
        }
    }

    async fn react_to_human_event(&self, human_event: HumanMessage) -> Result<(), SymbolError> {
        match human_event {
            HumanMessage::Anchor(anchor_request) => {
                let _ = self.handle_user_anchor_request(anchor_request).await;
            }
            HumanMessage::Followup(_followup_request) => {}
        }
        Ok(())
    }

    async fn human_message_anchor(
        &self,
        anchor_request: HumanAnchorRequest,
    ) -> Result<(), SymbolError> {
        let start_instant = std::time::Instant::now();
        println!("scratch_pad_agent::human_message_anchor::start");
        let anchored_symbols = anchor_request.anchored_symbols();
        let symbols_to_edit_request = self
            .tool_box
            .symbol_to_edit_request(
                anchored_symbols,
                anchor_request.user_query(),
                anchor_request.anchor_request_context(),
                self.message_properties.clone(),
            )
            .await?;

        let cloned_anchored_request = anchor_request.clone();
        // we are going to react to the user message
        let _ = self
            .reaction_sender
            .send(EnvironmentEventType::Human(HumanMessage::Anchor(
                cloned_anchored_request,
            )));

        // we start making the edits
        {
            let mut focussed = self.focussing.lock().await;
            *focussed = true;
        }
        let edits_done = stream::iter(symbols_to_edit_request.into_iter().map(|data| {
            (
                data,
                self.message_properties.clone(),
                self.symbol_event_sender.clone(),
            )
        }))
        .map(
            |(symbol_to_edit_request, message_properties, symbol_event_sender)| async move {
                let (sender, receiver) = tokio::sync::oneshot::channel();
                let symbol_event_request = SymbolEventRequest::new(
                    symbol_to_edit_request.symbol_identifier().clone(),
                    SymbolEvent::Edit(symbol_to_edit_request), // defines event type
                    ToolProperties::new(),
                );
                let event = SymbolEventMessage::message_with_properties(
                    symbol_event_request,
                    message_properties,
                    sender,
                );
                let _ = symbol_event_sender.send(event);
                receiver.await
            },
        )
        // run 100 edit requests in parallel to prevent race conditions
        .buffer_unordered(100)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .filter_map(|s| s.ok())
        .collect::<Vec<_>>();

        let cloned_user_query = anchor_request.user_query().to_owned();
        // the editor state has changed, so we need to react to that now
        let _ = self
            .reaction_sender
            .send(EnvironmentEventType::EditorStateChange(
                EditorStateChangeRequest::new(edits_done, cloned_user_query),
            ));
        // we are not focussed anymore, we can go about receiving events as usual
        {
            let mut focussed = self.focussing.lock().await;
            *focussed = false;
        }
        println!(
            "scratch_pad_agent::human_message_anchor::end::time_taken({}ms)",
            start_instant.elapsed().as_millis()
        );
        // send end of iteration event over here to the frontend
        let _ = self
            .message_properties
            .ui_sender()
            .send(UIEventWithID::code_iteration_finished(
                self.message_properties.request_id_str().to_owned(),
            ));
        Ok(())
    }

    async fn handle_user_anchor_request(&self, anchor_request: HumanAnchorRequest) {
        println!("scratch_pad::handle_user_anchor_request");
        // we are busy with the edits going on, so we can discard lsp signals for a while
        // figure out what to do over here
        let file_paths = anchor_request
            .anchored_symbols()
            .into_iter()
            .filter_map(|anchor_symbol| anchor_symbol.fs_file_path())
            .collect::<Vec<_>>();
        let mut already_seen_files: HashSet<String> = Default::default();
        let mut user_context_files = vec![];
        for fs_file_path in file_paths.into_iter() {
            if already_seen_files.contains(&fs_file_path) {
                continue;
            }
            already_seen_files.insert(fs_file_path.to_owned());
            let file_contents = self
                .tool_box
                .file_open(fs_file_path, self.message_properties.clone())
                .await;
            if let Ok(file_contents) = file_contents {
                user_context_files.push({
                    let file_path = file_contents.fs_file_path();
                    let language = file_contents.language();
                    let content = file_contents.contents_ref();
                    ScratchPadFilesActive::new(
                        format!(
                            r#"<file>
<fs_file_path>
{file_path}
</fs_file_path>
<content>
```{language}
{content}
```
</content>
</file>"#
                        ),
                        file_path.to_owned(),
                    )
                });
            }
        }
        // update our cache over here
        {
            let mut files_context = self.files_context.lock().await;
            *files_context = user_context_files.to_vec();
        }
        let user_context_files = user_context_files
            .into_iter()
            .map(|context_file| context_file.file_content)
            .collect::<Vec<_>>();
        println!("scratch_pad_agent::tool_box::agent_human_request");
        let _ = self
            .tool_box
            .scratch_pad_agent_human_request(
                self.storage_fs_path.to_owned(),
                anchor_request.user_query().to_owned(),
                user_context_files,
                anchor_request
                    .anchored_symbols()
                    .into_iter()
                    .map(|anchor_symbol| {
                        let content = anchor_symbol.content();
                        let fs_file_path = anchor_symbol.fs_file_path().unwrap_or_default();
                        let line_range_header = format!(
                            "{}-{}:{}",
                            fs_file_path,
                            anchor_symbol.possible_range().start_line(),
                            anchor_symbol.possible_range().end_line()
                        );
                        format!(
                            r#"Location: {line_range_header}
```
{content}
```"#
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
                self.message_properties.clone(),
            )
            .await;
    }

    /// We want to react to the various edits which have happened and the request they were linked to
    /// and come up with next steps and try to understand what we can do to help the developer
    async fn react_to_edits(&self, editor_state_change: EditorStateChangeRequest) {
        println!("scratch_pad::react_to_edits");
        // figure out what to do over here
        let user_context_files;
        {
            let files_context = self.files_context.lock().await;
            user_context_files = (*files_context).to_vec();
        }
        let _file_paths_in_focus = user_context_files
            .iter()
            .map(|context_file| context_file.file_path.to_owned())
            .collect::<HashSet<String>>();
        let user_context_files = user_context_files
            .into_iter()
            .map(|context_file| context_file.file_content)
            .collect::<Vec<_>>();
        let user_query = editor_state_change.user_query().to_owned();
        let edits_made = editor_state_change.consume_edits_made();
        let extra_context;
        {
            extra_context = (*self.extra_context.lock().await).to_owned();
        }
        {
            let mut extra_context = self.extra_context.lock().await;
            *extra_context = (*extra_context).to_owned()
                + &edits_made
                    .iter()
                    .map(|edit| edit.clone().to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
        }
        let _ = self
            .tool_box
            .scratch_pad_edits_made(
                &self.storage_fs_path,
                &user_query,
                &extra_context,
                edits_made
                    .into_iter()
                    .map(|edit| edit.to_string())
                    .collect::<Vec<_>>(),
                user_context_files,
                self.message_properties.clone(),
            )
            .await;

        // Now we want to grab the diagnostics which come in naturally
        // or via the files we are observing, there are race conditions here which
        // we want to tackle for sure
    }

    async fn react_to_ping(&self) -> Result<(), SymbolError> {
        println!("PINGED");
        let pad_contents = self
            .tool_box
            .file_open(
                self.storage_fs_path.to_owned(),
                self.message_properties.to_owned(),
            )
            .await?
            .contents();

        // asking the right questions.

        // let's go to LLM for tool use

        // use the tool

        Ok(())
    }

    /// We get to react to the lsp signal over here
    async fn react_to_lsp_signal(&self, lsp_signal: LSPSignal) {
        let focussed;
        {
            focussed = *(self.focussing.lock().await);
        }
        if focussed {
            return;
        }
        match lsp_signal {
            LSPSignal::Diagnostics(diagnostics) => {
                self.react_to_diagnostics(diagnostics).await;
            }
        }
    }

    async fn react_to_diagnostics(&self, diagnostics: Vec<LSPDiagnosticError>) {
        let file_paths_focussed;
        {
            file_paths_focussed = self
                .files_context
                .lock()
                .await
                .iter()
                .map(|file_content| file_content.file_path.to_owned())
                .collect::<HashSet<String>>();
        }
        let diagnostic_messages = diagnostics
            .into_iter()
            .filter(|diagnostic| file_paths_focussed.contains(diagnostic.fs_file_path()))
            .map(|diagnostic| {
                let diagnostic_file_path = diagnostic.fs_file_path();
                let diagnostic_message = diagnostic.diagnostic_message();
                format!(
                    r#"<fs_file_path>
{diagnostic_file_path}
</fs_file_path>
<message>
{diagnostic_message}
</message>"#
                )
            })
            .collect::<Vec<_>>();
        if diagnostic_messages.is_empty() {
            return;
        }
        println!("scratch_pad::reacting_to_diagnostics");
        let files_context;
        {
            files_context = (*self.files_context.lock().await).to_vec();
        }
        let extra_context;
        {
            extra_context = (*self.extra_context.lock().await).to_owned();
        }
        let _ = self
            .tool_box
            .scratch_pad_diagnostics(
                &self.storage_fs_path,
                diagnostic_messages,
                files_context
                    .into_iter()
                    .map(|files_context| files_context.file_content)
                    .collect::<Vec<_>>(),
                extra_context,
                self.message_properties.clone(),
            )
            .await;
    }
}
