use tonic::{transport::Server, Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;
use crate::application::application::Application;

pub mod proto {
    tonic::include_proto!("agent_farm");
}

fn convert_user_context(ctx: UserContext) -> crate::user_context::types::UserContext {
    crate::user_context::types::UserContext {
        repo_ref: ctx.repo_ref,
        precise_context: ctx.precise_context.into_iter().map(convert_precise_context).collect(),
        cursor_position: ctx.cursor_position.map(convert_cursor_position),
        current_view_port: ctx.current_view_port.map(convert_view_port),
        language: ctx.language,
    }
}

// Add conversion functions for other types...

use proto::agent_farm_service_server::{AgentFarmService, AgentFarmServiceServer};
use proto::*;

pub struct AgentFarmGrpcServer {
    app: Application,
}

#[tonic::async_trait]
impl AgentFarmService for AgentFarmGrpcServer {
    type AgentSessionChatStream = ReceiverStream<Result<AgentResponse, Status>>;
    type AgentSessionEditStream = ReceiverStream<Result<AgentEditResponse, Status>>;
    type AgentToolUseStream = ReceiverStream<Result<ToolUseResponse, Status>>;

    async fn agent_session_chat(
        &self,
        request: Request<AgentSessionRequest>,
    ) -> Result<Response<Self::AgentSessionChatStream>, Status> {
        let req = request.into_inner();
        let (tx, rx) = mpsc::channel(32);
        let app = self.app.clone();

        let llm_provider = LLMProperties::new(
            LLMType::ClaudeSonnet,
            LLMProvider::CodeStory(CodeStoryLLMTypes::new()),
            LLMProviderAPIKeys::CodeStory(CodestoryAccessToken::new(req.access_token.clone())),
        );

        let cancellation_token = tokio_util::sync::CancellationToken::new();
        let (ui_sender, ui_receiver) = tokio::sync::mpsc::unbounded_channel();
        let message_properties = SymbolEventMessageProperties::new(
            SymbolEventRequestId::new(req.thread_id.clone(), req.thread_id.clone()),
            ui_sender.clone(),
            req.editor_url.clone(),
            cancellation_token.clone(),
            llm_provider,
        );

        let session_storage_path = check_session_storage_path(app.config.clone(), req.thread_id.clone()).await;
        let session_service = app.session_service.clone();
        let cloned_thread_id = req.thread_id.clone();

        tokio::spawn(async move {
            let result = tokio::task::spawn(async move {
                session_service
                    .human_message(
                        cloned_thread_id.clone(),
                        session_storage_path,
                        req.thread_id.clone(),
                        req.user_query,
                        req.user_context.map(convert_user_context).unwrap_or_default(),
                        vec![], // project_labels
                        RepoRef::default(), // repo_ref
                        AideAgentMode::Chat,
                        req.aide_rules,
                        message_properties,
                    )
                    .await
            })
            .await;

            match result {
                Ok(Ok(_)) => (),
                Ok(Err(e)) => {
                    let error_msg = match e {
                        SymbolError::LLMClientError(LLMClientError::UnauthorizedAccess)
                        | SymbolError::ToolError(ToolError::LLMClientError(
                            LLMClientError::UnauthorizedAccess,
                        )) => "Unauthorized access. Please check your API key and try again.",
                        SymbolError::LLMClientError(LLMClientError::RateLimitExceeded)
                        | SymbolError::ToolError(ToolError::LLMClientError(
                            LLMClientError::RateLimitExceeded,
                        )) => "Rate limit exceeded. Please try again later.",
                        _ => "Internal server error",
                    };
                    let _ = tx.send(Ok(AgentResponse {
                        response: Some(agent_response::Response::Error(Error {
                            message: error_msg.to_string(),
                            kind: ErrorKind::Internal as i32,
                        })),
                    })).await;
                }
                Err(e) => {
                    let _ = tx.send(Ok(AgentResponse {
                        response: Some(agent_response::Response::Error(Error {
                            message: format!("Internal server error: {}", e),
                            kind: ErrorKind::Internal as i32,
                        })),
                    })).await;
                }
            }
        });

        // Convert UI events to AgentResponse
        tokio::spawn(async move {
            let mut ui_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(ui_receiver);
            while let Some(event) = ui_stream.next().await {
                match event {
                    UIEventWithID::Thinking { message, .. } => {
                        let _ = tx.send(Ok(AgentResponse {
                            response: Some(agent_response::Response::Thinking(message)),
                        })).await;
                    }
                    UIEventWithID::Action { message, .. } => {
                        let _ = tx.send(Ok(AgentResponse {
                            response: Some(agent_response::Response::Action(message)),
                        })).await;
                    }
                    UIEventWithID::Error { message, .. } => {
                        let _ = tx.send(Ok(AgentResponse {
                            response: Some(agent_response::Response::Error(Error {
                                message,
                                kind: ErrorKind::Internal as i32,
                            })),
                        })).await;
                    }
                    _ => {}
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn agent_session_edit(
        &self,
        request: Request<AgentEditRequest>,
    ) -> Result<Response<Self::AgentSessionEditStream>, Status> {
        let req = request.into_inner();
        let (tx, rx) = mpsc::channel(32);
        let app = self.app.clone();

        let session_storage_path = check_session_storage_path(app.config.clone(), req.thread_id.clone()).await;
        let scratch_pad_path = check_scratch_pad_path(app.config.clone(), req.thread_id.clone()).await;
        
        let scratch_pad_agent = ScratchPadAgent::new(
            scratch_pad_path,
            app.tool_box.clone(),
            app.symbol_manager.hub_sender(),
            None,
        ).await;

        let session_service = app.session_service.clone();
        let cloned_thread_id = req.thread_id.clone();

        tokio::spawn(async move {
            let result = tokio::task::spawn(async move {
                session_service
                    .code_edit_anchored(
                        cloned_thread_id.clone(),
                        session_storage_path,
                        scratch_pad_agent,
                        req.thread_id,
                        req.content,
                        req.context.map(convert_user_context).unwrap_or_default(),
                        req.aide_rules,
                        vec![], // project_labels
                        RepoRef::default(), // repo_ref
                        message_properties,
                    )
                    .await
            })
            .await;

            match result {
                Ok(Ok(_)) => (),
                Ok(Err(e)) => {
                    let error_msg = match e {
                        SymbolError::LLMClientError(LLMClientError::UnauthorizedAccess)
                        | SymbolError::ToolError(ToolError::LLMClientError(
                            LLMClientError::UnauthorizedAccess,
                        )) => "Unauthorized access. Please check your API key and try again.",
                        _ => "Internal server error",
                    };
                    let _ = tx.send(Ok(AgentEditResponse {
                        edited_content: String::new(),
                        modified_range: None,
                    })).await;
                }
                Err(e) => {
                    let _ = tx.send(Ok(AgentEditResponse {
                        edited_content: String::new(),
                        modified_range: None,
                    })).await;
                }
            }
        });

        // Convert UI events to AgentEditResponse
        tokio::spawn(async move {
            let mut ui_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(ui_receiver);
            while let Some(event) = ui_stream.next().await {
                match event {
                    UIEventWithID::FileEdit { content, range, .. } => {
                        let _ = tx.send(Ok(AgentEditResponse {
                            edited_content: content,
                            modified_range: Some(Range {
                                start: Some(Position {
                                    line: range.start().line() as u32,
                                    character: range.start().column() as u32,
                                }),
                                end: Some(Position {
                                    line: range.end().line() as u32,
                                    character: range.end().column() as u32,
                                }),
                            }),
                        })).await;
                    }
                    _ => {}
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn agent_tool_use(
        &self,
        request: Request<ToolUseRequest>,
    ) -> Result<Response<Self::AgentToolUseStream>, Status> {
        let req = request.into_inner();
        let (tx, rx) = mpsc::channel(32);
        let app = self.app.clone();

        let llm_provider = LLMProperties::new(
            LLMType::ClaudeSonnet,
            LLMProvider::CodeStory(CodeStoryLLMTypes::new()),
            LLMProviderAPIKeys::CodeStory(CodestoryAccessToken::new(req.access_token.clone())),
        );

        let cancellation_token = tokio_util::sync::CancellationToken::new();
        let (ui_sender, ui_receiver) = tokio::sync::mpsc::unbounded_channel();
        let message_properties = SymbolEventMessageProperties::new(
            SymbolEventRequestId::new(req.thread_id.clone(), req.thread_id.clone()),
            ui_sender.clone(),
            req.editor_url.clone(),
            cancellation_token.clone(),
            llm_provider,
        );

        let session_storage_path = check_session_storage_path(app.config.clone(), req.thread_id.clone()).await;
        let session_service = app.session_service.clone();
        let cloned_thread_id = req.thread_id.clone();

        tokio::spawn(async move {
            let result = tokio::task::spawn(async move {
                session_service
                    .tool_use_agentic(
                        cloned_thread_id.clone(),
                        session_storage_path,
                        req.tool_name,
                        req.parameters,
                        vec![], // all_files
                        vec![], // open_files
                        req.shell.unwrap_or_default(),
                        vec![], // project_labels
                        RepoRef::default(), // repo_ref
                        req.root_directory.unwrap_or_default(),
                        app.tool_box.clone(),
                        app.llm_broker.clone(),
                        req.context.map(convert_user_context).unwrap_or_default(),
                        req.aide_rules,
                        false, // reasoning
                        true,  // running in editor
                        false, // semantic_search
                        None,  // mcts_log_directory
                        None,  // repo_name
                        message_properties,
                        false, // is_devtools_context
                    )
                    .await
            })
            .await;

            match result {
                Ok(Ok(_)) => (),
                Ok(Err(e)) => {
                    let error_msg = match e {
                        SymbolError::LLMClientError(LLMClientError::UnauthorizedAccess)
                        | SymbolError::ToolError(ToolError::LLMClientError(
                            LLMClientError::UnauthorizedAccess,
                        )) => "Unauthorized access. Please check your API key and try again.",
                        SymbolError::LLMClientError(LLMClientError::RateLimitExceeded)
                        | SymbolError::ToolError(ToolError::LLMClientError(
                            LLMClientError::RateLimitExceeded,
                        )) => "Rate limit exceeded. Please try again later.",
                        _ => "Internal server error",
                    };
                    let _ = tx.send(Ok(ToolUseResponse {
                        result: String::new(),
                        success: false,
                        error: Some(error_msg.to_string()),
                    })).await;
                }
                Err(e) => {
                    let _ = tx.send(Ok(ToolUseResponse {
                        result: String::new(),
                        success: false,
                        error: Some(format!("Internal server error: {}", e)),
                    })).await;
                }
            }
        });

        // Convert UI events to ToolUseResponse
        tokio::spawn(async move {
            let mut ui_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(ui_receiver);
            while let Some(event) = ui_stream.next().await {
                match event {
                    UIEventWithID::ToolUse { result, success, .. } => {
                        let _ = tx.send(Ok(ToolUseResponse {
                            result,
                            success,
                            error: None,
                        })).await;
                    }
                    UIEventWithID::Error { message, .. } => {
                        let _ = tx.send(Ok(ToolUseResponse {
                            result: String::new(),
                            success: false,
                            error: Some(message),
                        })).await;
                    }
                    _ => {}
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn edit_file(
        &self,
        request: Request<EditFileRequest>,
    ) -> Result<Response<EditFileResponse>, Status> {
        let req = request.into_inner();
        
        match self.app.tool_box.file_edit(
            req.file_path,
            req.content,
            None,
        ).await {
            Ok(_) => Ok(Response::new(EditFileResponse {
                success: true,
                error: None,
            })),
            Err(e) => Ok(Response::new(EditFileResponse {
                success: false,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn extract_documentation(
        &self,
        request: Request<DocumentationRequest>,
    ) -> Result<Response<DocumentationResponse>, Status> {
        let req = request.into_inner();
        
        self.app.tool_box.extract_documentation(
            &req.file_content,
            &req.language,
        )
        .await
        .map(|docs| Response::new(DocumentationResponse {
            documentation_strings: docs,
        }))
        .map_err(convert_error_to_status)
    }

    async fn validate_tree_sitter(
        &self,
        request: Request<TreeSitterValidationRequest>,
    ) -> Result<Response<TreeSitterValidationResponse>, Status> {
        let req = request.into_inner();
        
        match self.app.tool_box.validate_tree_sitter(
            &req.content,
            &req.language,
        ).await {
            Ok(is_valid) => Ok(Response::new(TreeSitterValidationResponse {
                is_valid,
                error: None,
            })),
            Err(e) => Ok(Response::new(TreeSitterValidationResponse {
                is_valid: false,
                error: Some(e.to_string()),
            })),
        }
    }
}

impl AgentFarmGrpcServer {
    pub fn new(app: Application) -> Self {
        Self { app }
    }

    pub async fn serve(self, addr: std::net::SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        let svc = AgentFarmServiceServer::new(self);
        Server::builder()
            .add_service(svc)
            .serve(addr)
            .await?;
        Ok(())
    }
}