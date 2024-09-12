//! The scratchpad agent and the prompts for it
//! We are still not sure what this will look like, so consider everything over
//! here to be best effort

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage, LLMType},
    provider::{AnthropicAPIKey, LLMProvider, LLMProviderAPIKeys},
};

use crate::{
    agentic::{
        symbol::{
            identifier::SymbolIdentifier,
            ui_event::{EditedCodeStreamingRequest, UIEventWithID},
        },
        tool::{
            code_edit::search_and_replace::{
                SearchAndReplaceEditingResponse, StreamedEditingForEditor,
            },
            errors::ToolError,
            input::ToolInput,
            output::ToolOutput,
            r#type::Tool,
        },
    },
    chunking::text_document::{Position, Range},
};

/// Represents a broker for the ScratchPad Agent, responsible for managing interactions
/// with the language model and processing scratchpad-related tasks.
pub struct ScratchPadAgentBroker {
    /// The LLM (Language Model) client wrapped in an Arc for thread-safe sharing.
    llm_client: Arc<LLMBroker>,
}

impl ScratchPadAgentBroker {
    pub fn new(llm_client: Arc<LLMBroker>) -> Self {
        Self { llm_client }
    }
}

#[derive(Debug, Clone)]
pub struct ScratchPadAgentHumanMessage {
    user_code_context: String,
    user_context_files: Vec<String>,
    query: String,
}

impl ScratchPadAgentHumanMessage {
    pub fn new(user_code_context: String, user_context_files: Vec<String>, query: String) -> Self {
        Self {
            user_code_context,
            user_context_files,
            query,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScratchPadAgentEdits {
    edits_made: Vec<String>,
    user_request: String,
}

impl ScratchPadAgentEdits {
    pub fn new(edits_made: Vec<String>, user_request: String) -> Self {
        Self {
            edits_made,
            user_request,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScratchPadDiagnosticSignal {
    diagnostics: Vec<String>,
}

impl ScratchPadDiagnosticSignal {
    pub fn new(diagnostics: Vec<String>) -> Self {
        Self { diagnostics }
    }
}

#[derive(Debug, Clone)]
pub enum ScratchPadAgentInputType {
    UserMessage(ScratchPadAgentHumanMessage),
    EditsMade(ScratchPadAgentEdits),
    LSPDiagnosticMessage(ScratchPadDiagnosticSignal),
    CacheWarmup,
}

impl ScratchPadAgentInputType {
    fn is_cache_warmup(&self) -> bool {
        matches!(self, Self::CacheWarmup)
    }

    fn to_string(self) -> String {
        match self {
            Self::UserMessage(user_message) => {
                let files = user_message.user_context_files.join("\n");
                let user_query = user_message.query;
                let user_context = user_message.user_code_context;
                format!(
                    r#"I am looking at the following files
<files>
{files}
</files>

The code which I want to edit:
<code_in_selection>
{user_context}
</code_in_selection>

The changes I intend to do:
<query>
{user_query}
</query>"#
                )
            }
            Self::EditsMade(edits_made) => {
                let user_query = edits_made.user_request;
                let edits_made = edits_made.edits_made.join("\n");
                format!(
                    r#"I have made the following changes:
<changes>
{edits_made}
</changes>

and my intention was:
<query>
{user_query}
</query>"#
                )
            }
            Self::LSPDiagnosticMessage(diagnostics) => {
                let diagnostics = diagnostics.diagnostics.join("\n");
                format!(
                    r#"I can see the following diagnostic errors on the editor:
{diagnostics}"#
                )
            }
            Self::CacheWarmup => "".to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScratchPadAgentInput {
    files_context: Vec<String>,
    extra_context: String,
    input_event: ScratchPadAgentInputType,
    scratch_pad_content: String,
    scratch_pad_path: String,
    root_request_id: String,
    _ui_sender: UnboundedSender<UIEventWithID>,
    editor_url: String,
}

impl ScratchPadAgentInput {
    pub fn new(
        files_context: Vec<String>,
        extra_context: String,
        input_event: ScratchPadAgentInputType,
        scratch_pad_content: String,
        scratch_pad_path: String,
        root_request_id: String,
        ui_sender: UnboundedSender<UIEventWithID>,
        editor_url: String,
    ) -> Self {
        Self {
            files_context,
            extra_context,
            input_event,
            scratch_pad_content,
            scratch_pad_path,
            root_request_id,
            _ui_sender: ui_sender,
            editor_url,
        }
    }
}

struct ScratchPadAgentUserMessage {
    user_messages: Vec<LLMClientMessage>,
    is_cache_warmup: bool,
    root_request_id: String,
}

impl ScratchPadAgentBroker {
    fn system_message(&self) -> String {
        format!(
            r#"Act as an expert software engineer.
You are going to act as a second pair of eyes and brain for the developer working in a code editor.
You are not on the keyboard, but beside the developer who is going to go about making changes.
You are the pair-programmer to the developer and your goal is to help them out in the best possible ways.
Your task is to keep an eye on everything happening in the editor and come up with a TASK LIST to help the user.
You will be given a scratchpad which you can use to record the list of tasks which you believe the developer and you together should work on.
The scratchpad might be already populated with the tasks and the various states they were in before.

The scratchpad is a special place structured as following:
<scratchpad>
<files_visible>
</files_visible>
<tasks>
</tasks>
</scratchpad>

You are free to use the scratchpad as your notebook where you can record your work.
We explain each section of the scratchpad below:
- <files_visible>
These are the files which are visible to you in the editor, if you want to open new files or ask for more information please use the <next_steps> section and state the WHY always
- <tasks>
The tasks can be in 3 different modes:
- [in_progress] The inprogress tasks are the ones which are going on right now
- [blocked] The blocked tasks are the one which we can not do right now because either we do not have enough context or requires more effort than a simple edit in the current file. These can also be tasks which are incomplete
- [on_going] These are tasks which YOU want to do as they are easy and you want to help the developer, these tasks will be your responsibility so be very confident when you suggest this because you are going to take over the keyboard from the developer and the developer is going to watch you work.
These tasks contain the complete list which you and the developer will be working on, make sure you mark a task which is being worked on as [in_progress] (when the developer is working on it), if its completed mark it as [complete]. Keep this strucutred as a list (using -) and try to not repeat the same task again.
If the task has multiple steps, put them in a sub list indentended under the main task, for example:
- Example task
 - sub-task-1
 - sub-task-2
The developer also sees this and decides what they want to do next, so keep this VERY HIGH VALUE
If a particular task requires more effort or is still incomplete, mark it as [blocked] and in a sub-list describe in a single sentence why this is blocked.
The developer might go above and beyond and do extra work which might complete other parts of the tasks, be sure to keep the list of tasks as very high value with no repetitions.
Do not use vague tasks like: "check if its initialized properly" or "redo the documentation", these are low value and come in the way of the developer. Both you are developer are super smart so these obvious things are taken care of.

Examples of bad tasks which you should not list:
- Update the documentation (the developer and you are smart enough to never forget this)
- Unless told otherwise, do not worry about tests right now and create them as tasks

The different kind of signals which you get are of the following type:
- The user might have asked you for a question about some portion of the code.
- The user intends to edit some part of the codebase and they are telling you what they plan on doing, you should not suggest the edits since they will be done by the user, your job is to just observe the intention and help the developer understand if they missed anything.
- The edits which have been made could lead to additional change in the current file or files which are open in the editor.
- The editor has a language server running which generates diagnostic signals, its really important that you make sure to suggest tasks for these diagnostics.
- If you wish to go ahead and work on a task after reacting to a signal which you received, write it out and mark it as [on_going], you should be confident that you have all the context required to work on this task.
- If the task has been completed, spell out the code snippets which indicate why the task has been completed or the information which will help the developer understand that the task has been completed.

When coming up with the tasks, these are the tools inside the editor you have access to:
- Go-to-definition: This allows you to click on any code symbol and go to the definition of it, like the function call or the class definition
- Go-to-reference: This allows you to click on any code symbol and go to the references of the symbol
- Open-file: This allows you to open any file in the editor (you should use this if you are sure that such a path exists in the directory or you have high confidence about it)
So all your tasks should have sub-task list where each section either uses the above tool in some way, otherwise you can not proceed on the task.

Your scratchpad is a special place because the developer is also looking at it to inform themselves about the changes made to the codebase, so be concise and insightful in your scratchpad. Remember the developer trusts you a lot!

When you get a signal either from the developer or from the editor you must update the scratchpad, remember the developer is also using to keep an eye on the progress so be the most helpful pair-programmer you can be!
You have to generate the scratchpad again from scratch and rewrite the whole content which is present inside."#
        )
    }

    fn user_message(&self, input: ScratchPadAgentInput) -> ScratchPadAgentUserMessage {
        let files_context = input.files_context.join("\n");
        let extra_context = input.extra_context;
        let event_type = input.input_event;
        let scratch_pad_content = input.scratch_pad_content;
        let root_request_id = input.root_request_id;
        let is_cache_warmup = event_type.is_cache_warmup();
        let context_message = LLMClientMessage::user(format!(
            r#"I am providing you the files you asked for along with some extra context
<files_context>
{files_context}
</files_context>

<extra_context>
{extra_context}
</extra_context>

This is what I see in the scratchpad
{scratch_pad_content}"#
        ));
        let acknowledgment_message = LLMClientMessage::assistant("Thank you for providing me the additional context, I will keep this in mind when updating the scratchpad".to_owned()).cache_point();
        let user_message = if is_cache_warmup {
            event_type.to_string()
        } else {
            let event_type_str = event_type.to_string();
            format!(
                r#"{event_type_str}

As a reminder this is what you are supposed to do:
Act as an expert software engineer.
You are going to act as a second pair of eyes and brain for the developer working in a code editor.
You are not on the keyboard, but beside the developer who is going to go about making changes.
You are the pair-programmer to the developer and your goal is to help them out in the best possible ways.
Your task is to keep an eye on everything happening in the editor and come up with a TASK LIST to help the user.
You will be given a scratchpad which you can use to record the list of tasks which you believe the developer and you together should work on.
The scratchpad might be already populated with the tasks and the various states they were in before.

The scratchpad is a special place structured as following:
<scratchpad>
<files_visible>
</files_visible>
<tasks>
</tasks>
</scratchpad>

You are free to use the scratchpad as your notebook where you can record your work.
We explain each section of the scratchpad below:
- <files_visible>
These are the files which are visible to you in the editor, if you want to open new files or ask for more information please use the <next_steps> section and state the WHY always
- <tasks>
The tasks can be in 3 different modes:
- [in_progress] The inprogress tasks are the ones which are going on right now
- [blocked] The blocked tasks are the one which we can not do right now because either we do not have enough context or requires more effort than a simple edit in the current file. These can also be tasks which are incomplete
- [on_going] These are tasks which YOU want to do as they are easy and you want to help the developer, these tasks will be your responsibility so be very confident when you suggest this because you are going to take over the keyboard from the developer and the developer is going to watch you work.
These tasks contain the complete list which you and the developer will be working on, make sure you mark a task which is being worked on as [in_progress] (when the developer is working on it), if its completed mark it as [complete]. Keep this strucutred as a list (using -) and try to not repeat the same task again.
If the task has multiple steps, put them in a sub list indentended under the main task, for example:
- Example task
 - sub-task-1
 - sub-task-2
The developer also sees this and decides what they want to do next, so keep this VERY HIGH VALUE
If a particular task requires more effort or is still incomplete, mark it as [blocked] and in a sub-list describe in a single sentence why this is blocked.
The developer might go above and beyond and do extra work which might complete other parts of the tasks, be sure to keep the list of tasks as very high value with no repetitions.
Do not use vague tasks like: "check if its initialized properly" or "update the documentation", these are low value and come in the way of the developer. Both you are developer are super smart so these obvious things are taken care of.

Examples of bad tasks which you should not list:
- Update the documentation (the developer and you are smart enough to never forget this)
- Unless told otherwise, do not worry about tests right now and create them as tasks


The different kind of signals which you get are of the following type:
- The user might have asked you for a question about some portion of the code.
- The user intends to edit some part of the codebase and they are telling you what they plan on doing, you should not suggest the edits since they will be done by the user, your job is to just observe the intention and help the developer understand if they missed anything.
- The edits which have been made could lead to additional change in the current file or files which are open in the editor.
- The editor has a language server running which generates diagnostic signals, its really important that you make sure to suggest tasks for these diagnostics.
- If you wish to go ahead and work on a task after reacting to a signal which you received, write it out and mark it as [on_going], you should be confident that you have all the context required to work on this task.
- If the task has been completed, spell out the code snippets which indicate why the task has been completed or the information which will help the developer understand that the task has been completed.

When coming up with the tasks, these are the tools inside the editor you have access to:
- Go-to-definition: This allows you to click on any code symbol and go to the definition of it, like the function call or the class definition
- Go-to-reference: This allows you to click on any code symbol and go to the references of the symbol
- Open-file: This allows you to open any file in the editor (you should use this if you are sure that such a path exists in the directory or you have high confidence about it)
So all your tasks should have sub-task list where each section either uses the above tool in some way, otherwise you can not proceed on the task.

Your scratchpad is a special place because the developer is also looking at it to inform themselves about the changes made to the codebase, so be concise and insightful in your scratchpad. Remember the developer trusts you a lot!

When you get a signal either from the developer or from the editor you must update the scratchpad, remember the developer is also using to keep an eye on the progress so be the most helpful pair-programmer you can be!
You have to generate the scratchpad again from scratch and rewrite the whole content which is present inside.
Remember you have to reply only in the following format (do not deviate from this format at all!):
<scratchpad>
<files_visible>
</files_visible>
<tasks>
</tasks>
</scratchpad>"#
            )
        };
        ScratchPadAgentUserMessage {
            user_messages: vec![
                context_message,
                acknowledgment_message,
                LLMClientMessage::user(user_message),
            ],
            is_cache_warmup,
            root_request_id,
        }
    }
}

#[async_trait]
impl Tool for ScratchPadAgentBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        // figure out what to do over here
        println!("scratch_pad_agent_broker::invoked");
        let context = input.should_scratch_pad_input()?;
        let editor_url = context.editor_url.to_owned();
        let fs_file_path = context.scratch_pad_path.to_owned();
        let scratch_pad_range = Range::new(
            Position::new(0, 0, 0),
            Position::new(
                {
                    let lines = context
                        .scratch_pad_content
                        .lines()
                        .into_iter()
                        .collect::<Vec<_>>()
                        .len();
                    if lines == 0 {
                        0
                    } else {
                        lines - 1
                    }
                },
                1000,
                0,
            ),
        );
        let system_message = LLMClientMessage::system(self.system_message());
        let user_messages_context = self.user_message(context);
        let is_cache_warmup = user_messages_context.is_cache_warmup;
        let user_messages = user_messages_context.user_messages;
        let root_request_id = user_messages_context.root_request_id;
        let mut request = LLMClientCompletionRequest::new(
            LLMType::ClaudeSonnet,
            vec![system_message]
                .into_iter()
                .chain(user_messages)
                .collect::<Vec<_>>(),
            0.2,
            None,
        );
        if is_cache_warmup {
            request = request.set_max_tokens(1);
        }
        let api_key = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("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".to_owned()));
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let cloned_root_request_id = root_request_id.to_owned();
        let mut response = Box::pin(
            self.llm_client.stream_completion(
                api_key,
                request,
                LLMProvider::Anthropic,
                vec![
                    ("root_id".to_owned(), cloned_root_request_id),
                    ("event_type".to_owned(), "scratch_pad_agent".to_owned()),
                ]
                .into_iter()
                .collect(),
                sender,
            ),
        );
        if is_cache_warmup {
            println!("scratch_pad_agent::cache_warmup::skipping_early");
            return Ok(ToolOutput::SearchAndReplaceEditing(
                SearchAndReplaceEditingResponse::new("".to_owned(), "".to_owned()),
            ));
        }

        // we want to figure out how poll the llm stream while locking up until the file is free
        // from the lock over here for the file path we are interested in
        let edit_request_id = uuid::Uuid::new_v4().to_string();
        let _symbol_identifier = SymbolIdentifier::with_file_path(&fs_file_path, &fs_file_path);

        println!(
            "scratch_pad_agent::start_streaming::fs_file_path({})",
            &fs_file_path
        );
        let streamed_edit_client = StreamedEditingForEditor::new();
        // send a start event over here
        streamed_edit_client
            .send_edit_event(
                editor_url.to_owned(),
                EditedCodeStreamingRequest::start_edit(
                    edit_request_id.to_owned(),
                    scratch_pad_range.clone(),
                    fs_file_path.to_owned(),
                )
                .set_apply_directly(),
            )
            .await;
        streamed_edit_client
            .send_edit_event(
                editor_url.to_owned(),
                EditedCodeStreamingRequest::delta(
                    edit_request_id.to_owned(),
                    scratch_pad_range.clone(),
                    fs_file_path.to_owned(),
                    "```\n".to_owned(),
                )
                .set_apply_directly(),
            )
            .await;
        let stream_result;
        loop {
            tokio::select! {
                stream_msg = receiver.recv() => {
                    match stream_msg {
                        Some(msg) => {
                            let delta = msg.delta();
                            if let Some(delta) = delta {
                                let _ = streamed_edit_client.send_edit_event(
                                    editor_url.to_owned(),
                                    EditedCodeStreamingRequest::delta(
                                        edit_request_id.to_owned(),
                                        scratch_pad_range.clone(),
                                        fs_file_path.to_owned(),
                                        delta.to_owned(),
                                    ).set_apply_directly(),
                                ).await;
                            }
                        }
                        None => {
                            // something is up, the channel is closed? whatever
                        }
                    }
                }
                response = &mut response => {
                    if let Ok(_result) = response.as_deref() {
                        println!("scratch_pad_agent::stream_response::ok({:?})", _result);
                        let _ = streamed_edit_client.send_edit_event(
                            editor_url.to_owned(),
                            EditedCodeStreamingRequest::delta(
                                edit_request_id.to_owned(),
                                scratch_pad_range.clone(),
                                fs_file_path.to_owned(),
                                "\n```".to_owned(),
                            ).set_apply_directly()
                        ).await;
                        let _ = streamed_edit_client.send_edit_event(
                            editor_url.to_owned(),
                            EditedCodeStreamingRequest::end(
                                edit_request_id.to_owned(),
                                scratch_pad_range.clone(),
                                fs_file_path.to_owned(),
                            ).set_apply_directly()
                        ).await;
                    } else {
                        println!("scratch_pad_agent::stream_response::({:?})", response);
                        // send over the original selection over here since we had an error
                        let _ = streamed_edit_client.send_edit_event(
                            editor_url.to_owned(),
                            EditedCodeStreamingRequest::delta(
                                edit_request_id.to_owned(),
                                scratch_pad_range.clone(),
                                fs_file_path.to_owned(),
                                "\n```".to_owned(),
                            ).set_apply_directly()
                        ).await;
                        let _ = streamed_edit_client.send_edit_event(
                            editor_url.to_owned(),
                            EditedCodeStreamingRequest::end(
                                edit_request_id.to_owned(),
                                scratch_pad_range.clone(),
                                fs_file_path.to_owned(),
                            ).set_apply_directly()
                        ).await;
                    }
                    stream_result = Some(response);
                    break;
                }
            }
        }

        println!("scratch_pad::llm_response::({:?})", stream_result);

        match stream_result {
            Some(Ok(response)) => Ok(ToolOutput::SearchAndReplaceEditing(
                SearchAndReplaceEditingResponse::new(response.to_owned(), response.to_owned()),
            )),
            _ => Err(ToolError::MissingTool),
        }
    }
}
