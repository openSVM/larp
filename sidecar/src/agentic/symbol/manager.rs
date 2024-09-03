//! Contains the central manager for the symbols and maintains them in the system
//! as a connected graph in some ways in which these symbols are able to communicate
//! with each other

use std::sync::Arc;

use futures::{stream, StreamExt};
use llm_client::clients::types::LLMType;
use llm_client::provider::{GoogleAIStudioKey, LLMProvider};
use tokio::sync::mpsc::UnboundedSender;

use crate::agentic::swe_bench::search_cache::LongContextSearchCache;
use crate::agentic::symbol::events::initial_request::{InitialRequestData, SymbolEditedItem};
use crate::agentic::symbol::events::input::SymbolEventRequestId;
use crate::agentic::symbol::events::probe::SymbolToProbeRequest;
use crate::agentic::symbol::events::types::SymbolEvent;
use crate::agentic::symbol::helpers::SymbolFollowupBFS;
use crate::agentic::symbol::tool_properties::ToolProperties;
use crate::agentic::symbol::ui_event::InitialSearchSymbolInformation;
use crate::agentic::tool::code_symbol::important::CodeSymbolImportantWideSearch;
use crate::agentic::tool::input::ToolInput;
use crate::agentic::tool::r#type::Tool;
use crate::chunking::editor_parsing::EditorParsing;
use crate::chunking::languages::TSLanguageParsing;
use crate::user_context::types::UserContext;
use crate::{
    agentic::tool::{broker::ToolBroker, output::ToolOutput},
    inline_completion::symbols_tracker::SymbolTrackerInline,
};

use super::anchored::AnchoredSymbol;
use super::events::message_event::{SymbolEventMessage, SymbolEventMessageProperties};
use super::identifier::LLMProperties;
use super::tool_box::ToolBox;
use super::ui_event::UIEventWithID;
use super::{
    errors::SymbolError,
    events::input::SymbolInputEvent,
    locker::SymbolLocker,
    types::{SymbolEventRequest, SymbolEventResponse},
};

// This is the main communication manager between all the symbols
// this of this as the central hub through which all the events go forward
/// The SymbolManager is the central hub for managing and coordinating symbol-related operations.
/// It handles communication between symbols, manages their lifecycle, and orchestrates various tools and services.
pub struct SymbolManager {
    /// Channel sender for communication between symbols and the manager.
    /// This allows for asynchronous message passing within the system.
    sender: UnboundedSender<SymbolEventMessage>,

    /// Manages locking and unlocking of symbols to prevent concurrent access.
    /// This ensures thread-safety when multiple operations are performed on symbols simultaneously.
    symbol_locker: SymbolLocker,

    /// Broker for managing and invoking various tools.
    /// This provides a centralized way to access and use different tools required for symbol operations.
    tools: Arc<ToolBroker>,

    /// Parser for TypeScript language constructs.
    /// This is used to analyze and understand TypeScript code structures.
    ts_parsing: Arc<TSLanguageParsing>,

    /// Collection of tools and utilities for symbol operations.
    /// This provides a set of helper functions and utilities specific to symbol manipulation and analysis.
    tool_box: Arc<ToolBox>,

    /// Properties for the Language Model being used.
    /// This contains configuration and settings for the LLM used in various operations.
    llm_properties: LLMProperties,

    /// Cache for storing long-context search results.
    /// This improves performance by storing and reusing results of expensive long-context searches.
    long_context_cache: LongContextSearchCache,
}

impl SymbolManager {
    pub fn new(
        tools: Arc<ToolBroker>,
        symbol_broker: Arc<SymbolTrackerInline>,
        editor_parsing: Arc<EditorParsing>,
        llm_properties: LLMProperties,
    ) -> Self {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<SymbolEventMessage>();
        let tool_box = Arc::new(ToolBox::new(
            tools.clone(),
            symbol_broker.clone(),
            editor_parsing.clone(),
        ));
        let symbol_locker =
            SymbolLocker::new(sender.clone(), tool_box.clone(), llm_properties.clone());
        let cloned_symbol_locker = symbol_locker.clone();
        tokio::spawn(async move {
            // TODO(skcd): Make this run in full parallelism in the future, for
            // now this is fine
            while let Some(event) = receiver.recv().await {
                println!("symbol_manager::tokio::spawn::receiver_event");
                // let _ = cloned_ui_sender.send(UIEvent::from(event.0.clone()));
                let _ = cloned_symbol_locker.process_request(event).await;
            }
            println!("symbol_manager::tokio::spawn::end");
        });
        let ts_parsing = Arc::new(TSLanguageParsing::init());
        Self {
            sender,
            symbol_locker,
            ts_parsing,
            tools,
            tool_box,
            llm_properties,
            long_context_cache: LongContextSearchCache::new(),
        }
    }

    pub fn hub_sender(&self) -> UnboundedSender<SymbolEventMessage> {
        self.sender.clone()
    }

    pub async fn anchor_edits(
        &self,
        user_query: String,
        anchored_symbols: Vec<AnchoredSymbol>,
        user_provided_context: Option<String>,
        message_properties: SymbolEventMessageProperties,
    ) -> Result<(), SymbolError> {
        let symbols_to_edit_request = self
            .tool_box
            .symbol_to_edit_request(
                anchored_symbols,
                &user_query,
                user_provided_context,
                message_properties.clone(),
            )
            .await?;

        // Now we can send over these requests to the symbol locker to manager
        let _ = stream::iter(
            symbols_to_edit_request
                .into_iter()
                .map(|data| (data, message_properties.clone())),
        )
        .map(|(symbol_to_edit_request, message_properties)| async {
            let (sender, receiver) = tokio::sync::oneshot::channel();
            let symbol_event_request = SymbolEventRequest::new(
                symbol_to_edit_request.symbol_identifier().clone(),
                SymbolEvent::Edit(symbol_to_edit_request),
                ToolProperties::new(),
            );
            let event = SymbolEventMessage::message_with_properties(
                symbol_event_request,
                message_properties,
                sender,
            );
            let _ = self.symbol_locker.process_request(event).await;
            receiver.await
        })
        // run 1 edit requests in parallel to prevent race conditions
        .buffer_unordered(1)
        .collect::<Vec<_>>()
        .await;
        let _ = message_properties
            .ui_sender()
            .send(UIEventWithID::code_iteration_finished(
                message_properties.request_id_str().to_owned(),
            ));
        Ok(())
    }

    // TODO(codestory): This is hardcoded function, we of course want to follow
    // something similar but make it more generic later on
    pub async fn impls_test(
        &self,
        root_dir: &str,
        fs_file_path: &str,
        // this contains all the request id related jazz over here
        message_properties: SymbolEventMessageProperties,
    ) -> Result<(), SymbolError> {
        let symbol_change_set = self
            .tool_box
            .grab_changed_symbols_in_file_git(&root_dir, &fs_file_path)
            .await?;
        println!("symbol_change_set:{:?}", &symbol_change_set);

        let symbols_to_followup_request = symbol_change_set
            .remove_changes()
            .into_iter()
            .map(|changes| {
                let symbol_identifier = changes.symbol_identifier().clone();
                let symbol_identifier_ref = &symbol_identifier;
                changes
                    .remove_changes()
                    .into_iter()
                    .map(|symbol_to_edit| {
                        SymbolFollowupBFS::new(
                            symbol_to_edit.0,
                            symbol_identifier_ref.clone(),
                            symbol_to_edit.1,
                            symbol_to_edit.2,
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect::<Vec<_>>();

        // This needs to be an iterative loop over here instead of a single call
        // we have to perform a BFS at this layer so the symbols can organise and
        // get the job done
        let _ = self
            .tool_box
            .check_for_followups_bfs(
                symbols_to_followup_request,
                self.symbol_locker.hub_sender.clone(),
                message_properties,
                &ToolProperties::new().set_apply_edits_directly(),
            )
            .await;

        Ok(())
    }

    pub fn tool_box(&self) -> &ToolBox {
        &self.tool_box
    }

    pub async fn probe_request_from_user_context(
        &self,
        query: String,
        user_context: UserContext,
        message_properties: SymbolEventMessageProperties,
    ) -> Result<(), SymbolError> {
        println!("symbol_manager::probe_request_from_user_context::start");
        let request_id = uuid::Uuid::new_v4().to_string();
        let request_id = SymbolEventRequestId::new(request_id.to_owned(), request_id.to_owned());
        let request_id_ref = &request_id;
        let variables = self
            .tool_box
            .grab_symbols_from_user_context(
                query.to_owned(),
                user_context.clone(),
                message_properties.clone(),
            )
            .await;
        let outline = self
            .tool_box
            .outline_for_user_context(&user_context, message_properties.clone())
            .await;
        let code_wide_search =
            ToolInput::RequestImportantSymbolsCodeWide(CodeSymbolImportantWideSearch::new(
                user_context.clone(),
                query.to_owned(),
                // Hardcoding here, but we can remove this later
                LLMType::GeminiPro,
                LLMProvider::GoogleAIStudio,
                llm_client::provider::LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new(
                    "AIzaSyCMkKfNkmjF8rTOWMg53NiYmz0Zv6xbfsE".to_owned(),
                )),
                request_id_ref.root_request_id().to_owned(),
                outline,
            ));
        let output = {
            match variables {
                Ok(variables) => ToolOutput::ImportantSymbols(variables),
                _ => self
                    .tools
                    .invoke(code_wide_search)
                    .await
                    .map_err(|e| SymbolError::ToolError(e))?,
            }
        };
        println!(
            "symbol_manager::probe_request_from_user_context::output({:?})",
            &output
        );
        if let ToolOutput::ImportantSymbols(important_symbols)
        | ToolOutput::RepoMapSearch(important_symbols) = output
        {
            // We have the important symbols here which we can then use to invoke the individual process request
            let important_symbols = important_symbols.fix_symbol_names(self.ts_parsing.clone());

            let mut symbols = self
                .tool_box
                .important_symbols(&important_symbols, message_properties.clone())
                .await
                .map_err(|e| e.into())?;
            // TODO(skcd): Another check over here is that we can search for the exact variable
            // and then ask for the edit
            println!(
                "symbol_manager::probe_request_from_user_context::[{}]",
                symbols
                    .iter()
                    .map(|(symbol, _)| symbol.symbol_name().to_owned())
                    .collect::<Vec<_>>()
                    .join(",")
            );
            // TODO(skcd): the symbol here might belong to a class or it might be a global function
            // we want to grab the largest node containing the symbol here instead of using
            // the symbol directly since our algorithm would not work otherwise
            // we would also need to de-duplicate the symbols which we have to process right over here
            // otherwise it might lead to errors
            if symbols.is_empty() {
                println!("symbol_manager::grab_symbols_using_search");
                symbols = self
                    .tool_box
                    .grab_symbol_using_search(important_symbols, message_properties.clone())
                    .await
                    .map_err(|e| e.into())?;
            }

            // Create all the symbol agents
            let symbol_identifiers = stream::iter(
                symbols
                    .into_iter()
                    .map(|symbol| (symbol, message_properties.clone())),
            )
            .map(|((symbol_request, _), message_properties)| async move {
                let symbol_identifier = self
                    .symbol_locker
                    .create_symbol_agent(symbol_request, ToolProperties::new(), message_properties)
                    .await;
                symbol_identifier
            })
            .buffer_unordered(100)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .filter_map(|s| s.ok())
            .collect::<Vec<_>>();

            println!(
                "symbol_manager::probe_request_from_user_context::len({})",
                symbol_identifiers.len(),
            );

            let query_ref = &query;

            // Now for all the symbol identifiers which we are getting we have to
            // send a request to all of them with the probe query
            let responses = stream::iter(symbol_identifiers.into_iter().map(|symbol_identifier| {
                (
                    symbol_identifier.clone(),
                    SymbolEventRequest::probe_request(
                        symbol_identifier.clone(),
                        SymbolToProbeRequest::new(
                            symbol_identifier,
                            query_ref.to_owned(),
                            query_ref.to_owned(),
                            request_id_ref.root_request_id().to_owned(),
                            vec![],
                        ),
                        ToolProperties::new(),
                    ),
                    message_properties.clone(),
                )
            }))
            .map(
                |(symbol_identifier, symbol_event_request, message_properties)| async move {
                    let (sender, receiver) = tokio::sync::oneshot::channel();
                    dbg!(
                        "sending initial request to symbol: {:?}",
                        &symbol_identifier
                    );
                    let request_event = SymbolEventMessage::message_with_properties(
                        symbol_event_request,
                        message_properties.clone(),
                        sender,
                    );
                    self.symbol_locker.process_request(request_event).await;
                    let response = receiver.await;
                    dbg!(
                        "For symbol identifier: {:?} the response is {:?}",
                        &symbol_identifier,
                        &response
                    );
                    (response, symbol_identifier)
                },
            )
            .buffer_unordered(100)
            .collect::<Vec<_>>()
            .await;

            // send the response forward after combining all the answers using the LLM
            let final_responses = responses
                .into_iter()
                .filter_map(|(response, symbol_identifier)| match response {
                    Ok(response) => Some((response, symbol_identifier)),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let final_answer = final_responses
                .into_iter()
                .map(|(response, symbol_identifier)| {
                    let symbol_name = symbol_identifier.symbol_name();
                    let symbol_file_path = symbol_identifier
                        .fs_file_path()
                        .map(|fs_file_path| format!("at {}", fs_file_path))
                        .unwrap_or("".to_owned());
                    let response = response.to_string();
                    let symbol_readable = format!("{symbol_name} {symbol_file_path}");
                    format!(
                        r#"{symbol_readable}
{response}"#
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            let _ = message_properties
                .ui_sender()
                .send(UIEventWithID::probing_finished_event(
                    request_id_ref.root_request_id().to_owned(),
                    final_answer,
                ));
            println!("things are completed over here after probing");
        }
        println!("things are more complete over here after probing");
        Ok(())
    }

    // This is just for testing out the flow for single input events
    pub async fn probe_request(
        &self,
        symbol_event_message: SymbolEventMessage,
        receiver: tokio::sync::oneshot::Receiver<SymbolEventResponse>,
    ) -> Result<(), SymbolError> {
        let _ = self
            .symbol_locker
            .process_request(symbol_event_message)
            .await;
        let response = receiver.await;
        println!("{:?}", response.expect("to work"));
        Ok(())
    }

    // once we have the initial request, which we will go through the initial request
    // mode once, we have the symbols from it we can use them to spin up sub-symbols as well
    pub async fn initial_request(
        &self,
        input_event: SymbolInputEvent,
        message_properties: SymbolEventMessageProperties,
    ) -> Result<(), SymbolError> {
        let ui_sender = input_event.ui_sender();
        let request_id = input_event.request_id().to_owned();
        let is_full_edit = input_event.full_symbol_edit();
        let is_big_search = input_event.big_search();
        let swe_bench_id = input_event.swe_bench_instance_id();
        let swe_bench_git_dname = input_event.get_swe_bench_git_dname();
        let swe_bench_test_endpoint = input_event.get_swe_bench_test_endpoint();
        let swe_bench_code_editing_model = input_event.get_swe_bench_code_editing();
        let swe_bench_gemini_properties = input_event.get_swe_bench_gemini_llm_properties();
        let swe_bench_long_context_editing = input_event.get_swe_bench_long_context_editing();
        let full_symbol_edit = input_event.full_symbol_edit();
        let fast_code_symbol_llm = input_event.get_fast_code_symbol_llm();
        let tool_properties = ToolProperties::new()
            .set_swe_bench_endpoint(swe_bench_test_endpoint.clone())
            .set_swe_bench_code_editing_llm(swe_bench_code_editing_model)
            .set_swe_bench_reranking_llm(swe_bench_gemini_properties)
            .set_long_context_editing_llm(swe_bench_long_context_editing)
            .set_full_symbol_request(full_symbol_edit)
            .set_fast_code_symbol_search(fast_code_symbol_llm);
        let user_query = input_event.user_query().to_owned();
        let tool_input = input_event
            .tool_use_on_initial_invocation(self.tool_box.clone(), message_properties.clone())
            .await;
        if let Some(tool_input) = tool_input {
            // send the tool input to the ui sender
            // we need some kind of cache here so we do not invoke the expensive
            // query so many times
            let important_symbols = if let Some(swe_bench_id) = swe_bench_id.to_owned() {
                let symbols = self.long_context_cache.check_cache(&swe_bench_id).await;
                if let Some(_git_dname) = swe_bench_git_dname {
                    match symbols {
                        Some(symbols) => Some(symbols),
                        None => None,
                    }
                } else {
                    symbols
                }
            } else {
                if request_id == "testing_code_editing_flow" {
                    self.long_context_cache.check_cache(&request_id).await
                } else {
                    None
                }
            };

            let tool_output = match important_symbols {
                Some(important_symbols) => ToolOutput::RepoMapSearch(important_symbols),
                None => {
                    if tool_input.is_repo_map_search() {
                        let _ = ui_sender.send(UIEventWithID::start_long_context_search(
                            request_id.to_owned(),
                        ));
                        let result = self
                            .tools
                            .invoke(tool_input)
                            .await
                            .map_err(|e| SymbolError::ToolError(e));
                        let _ = ui_sender.send(UIEventWithID::finish_long_context_search(
                            request_id.to_owned(),
                        ));
                        result?
                    } else {
                        println!("invoking for tool_output - requisite for important_symbols");
                        self.tools
                            .invoke(tool_input)
                            .await
                            .map_err(|e| SymbolError::ToolError(e))?
                    }
                }
            };

            if let ToolOutput::ImportantSymbols(important_symbols)
            | ToolOutput::RepoMapSearch(important_symbols)
            | ToolOutput::BigSearch(important_symbols) = tool_output
            {
                // The fix symbol name here helps us get the top-level symbol name
                // if the LLM decides to have fun and spit out a.b.c instead of a or b or c individually
                // as it can with python where it will tell class.method_name instead of just class or just
                // method_name

                // should pass self.editorparsing <tsconfigs>
                let important_symbols = important_symbols.fix_symbol_names(self.ts_parsing.clone());
                // swe bench caching hit over here we just do it
                self.long_context_cache
                    .update_cache(swe_bench_id.clone(), &important_symbols)
                    .await;
                // TODO(codestory): Remove this after testing
                if request_id == "testing_code_editing_flow" {
                    let _ = self
                        .long_context_cache
                        .update_cache(Some(request_id.to_owned()), &important_symbols)
                        .await;
                }

                // Debug printing
                println!("Important symbols: {:?}", &important_symbols);

                println!("symbol_manager::planning_before_editing");
                let important_symbols = match self
                    .long_context_cache
                    .check_cache_for_plan_before_editing(
                        swe_bench_id
                            .clone()
                            .map(|_swe_bench_id| request_id.to_owned()),
                    )
                    .await
                {
                    Some(important_symbols) => important_symbols,
                    None => {
                        if is_full_edit && !is_big_search {
                            important_symbols
                        } else {
                            let important_symbols = self
                                .tool_box
                                .planning_before_code_editing(
                                    &important_symbols,
                                    &user_query,
                                    self.llm_properties.clone(),
                                    is_big_search,
                                    message_properties.clone(),
                                )
                                .await?
                                .fix_symbol_names(self.ts_parsing.clone());
                            self.long_context_cache
                                .update_cache_for_plan_before_editing(
                                    swe_bench_id.map(|_swe_bench_id| request_id.to_owned()),
                                    &important_symbols,
                                )
                                .await;
                            important_symbols
                        }
                    }
                };

                println!("symbol_manager::plan_finished_before_editing");

                let updated_tool_properties = important_symbols.ordered_symbols_to_plan();
                let tool_properties = tool_properties
                    .clone()
                    .set_plan_for_input(Some(updated_tool_properties));
                let tool_properties_ref = &tool_properties;
                println!(
                    "symbol_manager::tool_box::important_symbols::search({})",
                    important_symbols
                        .ordered_symbols()
                        .into_iter()
                        .map(|code_symbol| code_symbol.code_symbol())
                        .collect::<Vec<_>>()
                        .join(",")
                );

                // Lets first start another round of COT over here to figure out
                // how to go about making the changes, I know this is a bit orthodox
                // and goes against our plans of making the agents better, but
                // this feels useful to have, since the previous iteration
                // does not even see the code and what changes need to be made
                let mut symbols = self
                    .tool_box
                    .important_symbols(&important_symbols, message_properties.clone())
                    .await
                    .map_err(|e| e.into())?;

                // send a UI event to the frontend over here
                let mut initial_symbol_search_information = vec![];
                for (symbol, _) in symbols.iter() {
                    initial_symbol_search_information.push(InitialSearchSymbolInformation::new(
                        symbol.symbol_name().to_owned(),
                        // TODO(codestory): umm.. how can we have a file path for a symbol
                        // which does not exist if is_new is true
                        Some(symbol.fs_file_path().to_owned()),
                        symbol.is_new(),
                        symbol.steps().await.join("\n"),
                        symbol
                            .get_snippet()
                            .await
                            .map(|snippet| snippet.range().clone()),
                    ));
                }
                let _ = ui_sender.send(UIEventWithID::initial_search_symbol_event(
                    request_id.to_owned(),
                    initial_symbol_search_information,
                ));
                // TODO(skcd): Another check over here is that we can search for the exact variable
                // and then ask for the edit
                println!(
                    "symbol_manager::initial_request::symbols::({})",
                    symbols
                        .iter()
                        .map(|(symbol, _)| symbol.symbol_name().to_owned())
                        .collect::<Vec<_>>()
                        .join(",")
                );

                let symbols_edited_list = important_symbols
                    .ordered_symbols()
                    .into_iter()
                    .map(|symbol| {
                        SymbolEditedItem::new(
                            symbol.code_symbol().to_owned(),
                            symbol.file_path().to_owned(),
                            symbol.is_new(),
                            symbol.steps().to_vec().join("\n"),
                        )
                    })
                    .collect::<Vec<_>>();
                // TODO(skcd): the symbol here might belong to a class or it might be a global function
                // we want to grab the largest node containing the symbol here instead of using
                // the symbol directly since our algorithm would not work otherwise
                // we would also need to de-duplicate the symbols which we have to process right over here
                // otherwise it might lead to errors
                if symbols.is_empty() {
                    println!("symbol_manager::grab_symbols_using_search");
                    symbols = self
                        .tool_box
                        .grab_symbol_using_search(important_symbols, message_properties.clone())
                        .await
                        .map_err(|e| e.into())?;
                }

                println!("symbol_manager::symbols_len::({})", symbols.len());

                // This is where we are creating all the symbols
                let _ = stream::iter(
                    // we are loosing context about the changes which we want to make
                    // to the symbol over here
                    symbols.into_iter().map(|symbol| {
                        (
                            symbol,
                            user_query.to_owned(),
                            symbols_edited_list.to_vec(),
                            message_properties.clone(),
                        )
                    }),
                )
                .map(
                    |(
                        (symbol_request, steps),
                        user_query,
                        symbols_edited_list,
                        message_properties,
                    )| async move {
                        let symbol_name = symbol_request.symbol_name().to_owned();
                        let symbol_identifier = self
                            .symbol_locker
                            .create_symbol_agent(
                                symbol_request,
                                tool_properties_ref.clone(),
                                message_properties.clone(),
                            )
                            .await;
                        if let Ok(symbol_identifier) = symbol_identifier {
                            let symbol_event_request = SymbolEventRequest::new(
                                symbol_identifier.clone(),
                                SymbolEvent::InitialRequest(InitialRequestData::new(
                                    user_query.to_owned(),
                                    steps.join("\n"),
                                    // empty history when symbol manager sends the initial
                                    // request
                                    vec![],
                                    full_symbol_edit,
                                    Some(symbols_edited_list),
                                    is_big_search,
                                )),
                                tool_properties_ref.clone(),
                            );
                            let (sender, receiver) = tokio::sync::oneshot::channel();
                            println!(
                                "symbol_manager::initial_request::sending_request({})",
                                symbol_identifier.symbol_name()
                            );
                            let symbol_event = SymbolEventMessage::message_with_properties(
                                symbol_event_request,
                                message_properties.clone(),
                                sender,
                            );
                            self.symbol_locker.process_request(symbol_event).await;
                            let response = receiver.await;
                            println!(
                                "symbol_manager::initial_request::response::({})::({:?})",
                                symbol_identifier.symbol_name(),
                                &response
                            );
                        } else {
                            println!(
                                "symbol_manager::initial_request::no_symbol_identifier({})",
                                symbol_name
                            );
                        }
                    },
                )
                .buffered(1)
                .collect::<Vec<_>>()
                .await;
            }
        } else {
            // We are for some reason not even invoking the first passage which is
            // weird, this is completely wrong and we should be replying back
            println!("this is wrong, please look at the comment here");
        }
        println!("symbol_manager::initial_request::finish");
        let _ = ui_sender.send(UIEventWithID::finish_edit_request(request_id));
        Ok(())
    }
}
