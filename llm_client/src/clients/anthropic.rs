    fn format_message_content(message: &super::types::LLMClientMessage) -> String {
        format!(
            r#"<content>
{}
</content>
<tool_use_value>
{}
</tool_use_value>
<tool_return_value>
{}
</tool_return_value>"#,
            message.content(),
            message.tool_use_value().into_iter()
                .filter_map(|v| serde_json::to_string(&v).ok())
                .collect::<Vec<_>>()
                .join("\n"),
            message.tool_return_value().into_iter()
                .filter_map(|v| serde_json::to_string(&v).ok())
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    fn format_completion_content(
        content: &str,
        tool_use: &[(String, (String, String))],
    ) -> String {
        format!(
            "<content>\n{}\n</content>\n<tool_use_indication>\n{}\n</tool_use_indication>",
            content,
            tool_use.iter()
                .map(|(_, (tool_type, tool_value))| {
                    format!(
                        "<tool_use_value>\n<tool_type>\n{}\n</tool_type>\n<tool_content>\n{}\n</tool_content>\n</tool_use_value>",
                        tool_type, tool_value
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use logging::new_client;
use logging::parea::{PareaClient, PareaLogCompletion, PareaLogMessage};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};

use crate::{
    clients::types::LLMClientUsageStatistics,
    provider::{LLMProvider, LLMProviderAPIKeys},
};

use super::types::{
    LLMClient, LLMClientCompletionRequest, LLMClientCompletionResponse,
    LLMClientCompletionStringRequest, LLMClientError, LLMClientMessageImage, LLMClientToolReturn,
    LLMClientToolUse, LLMType,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum AnthropicCacheType {
    #[serde(rename = "ephemeral")]
    Ephemeral,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct AnthropicCacheControl {
    r#type: AnthropicCacheType,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum AnthropicMessageContent {
    #[serde(rename = "text")]
    Text {
        text: String,
        cache_control: Option<AnthropicCacheControl>,
    },
    #[serde(rename = "image")]
    Image {
        source: AnthropicImageSource,
        cache_control: Option<AnthropicCacheControl>,
    },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
        cache_control: Option<AnthropicCacheControl>,
    },
    #[serde(rename = "tool_result")]
    ToolReturn {
        tool_use_id: String,
        content: String,
        cache_control: Option<AnthropicCacheControl>,
    },
}

impl AnthropicMessageContent {
    pub fn text(content: String, cache_control: Option<AnthropicCacheControl>) -> Self {
        Self::Text {
            text: content,
            cache_control,
        }
    }

    pub fn image(llm_image: &LLMClientMessageImage) -> Self {
        Self::Image {
            source: AnthropicImageSource {
                r#type: llm_image.r#type().to_owned(),
                media_type: llm_image.media().to_owned(),
                data: llm_image.data().to_owned(),
            },
            cache_control: None,
        }
    }

    pub fn tool_use(llm_tool_use: &LLMClientToolUse) -> Self {
        Self::ToolUse {
            id: llm_tool_use.id().to_owned(),
            name: llm_tool_use.name().to_owned(),
            input: llm_tool_use.input().clone(),
            cache_control: None,
        }
    }

    pub fn tool_return(llm_tool_return: &LLMClientToolReturn) -> Self {
        Self::ToolReturn {
            tool_use_id: llm_tool_return.tool_use_id().to_owned(),
            content: llm_tool_return.content().to_owned(),
            cache_control: None,
        }
    }

    fn with_cache_control(self, is_cache_point: bool) -> Self {
        if !is_cache_point {
            return self;
        }
        self.cache_control(Some(AnthropicCacheControl {
            r#type: AnthropicCacheType::Ephemeral,
        }))
    }

    fn cache_control(mut self, cache_control_update: Option<AnthropicCacheControl>) -> Self {
        match &mut self {
            Self::Text { cache_control, .. } |
            Self::Image { cache_control, .. } |
            Self::ToolUse { cache_control, .. } |
            Self::ToolReturn { cache_control, .. } => {
                *cache_control = cache_control_update;
            }
        }
        self
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct AnthropicImageSource {
    r#type: String,     // e.g., "base64"
    media_type: String, // e.g., "image/png"
    data: String,       // base64-encoded image data
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicMessageContent>,
}

impl AnthropicMessage {
    fn collect_content(message: &super::types::LLMClientMessage) -> Vec<AnthropicMessageContent> {
        let mut content = Vec::new();
        
        // Add text content if we don't have tool returns
        if message.tool_return_value().is_empty() && !message.content().is_empty() {
            content.push(AnthropicMessageContent::text(message.content().to_owned(), None));
        }
        
        // Add images, tools and tool returns
        content.extend(message.images().iter().map(AnthropicMessageContent::image));
        content.extend(message.tool_use_value().iter().map(AnthropicMessageContent::tool_use));
        content.extend(message.tool_return_value().iter().map(AnthropicMessageContent::tool_return));

        // Apply cache control to last content if needed
        if message.is_cache_point() && !content.is_empty() {
            let last_idx = content.len() - 1;
            content[last_idx] = content[last_idx].clone().with_cache_control(true);
        }

        content
    }

    fn handle_event_content(
        event: AnthropicEvent,
        buffered_string: &mut String,
        model_str: &str,
        sender: &UnboundedSender<LLMClientCompletionResponse>,
        usage_stats: Option<LLMClientUsageStatistics>,
    ) -> Result<(), LLMClientError> {
        match event {
            AnthropicEvent::ContentBlockStart { content_block, .. } => match content_block {
                ContentBlockStart::InputToolUse { name, .. } => {
                    info!("anthropic::tool_use::{}", &name);
                    Ok(())
                }
                ContentBlockStart::TextDelta { text } => {
                    *buffered_string += &text;
                    Self::send_completion_response(buffered_string, &text, model_str, sender, usage_stats)
                }
            },
            AnthropicEvent::ContentBlockDelta { delta, .. } => match delta {
                ContentBlockDeltaType::TextDelta { text } => {
                    *buffered_string += &text;
                    Self::send_completion_response(buffered_string, &text, model_str, sender, usage_stats)
                }
                ContentBlockDeltaType::InputJsonDelta { partial_json } => {
                    debug!("input_json_delta::{}", &partial_json);
                    Ok(())
                }
            },
            _ => Ok(()),
        }
    }

    fn send_completion_response(
        buffered_string: &str,
        text: &str,
        model_str: &str,
        sender: &UnboundedSender<LLMClientCompletionResponse>,
        usage_stats: Option<LLMClientUsageStatistics>,
    ) -> Result<(), LLMClientError> {
        let mut response = LLMClientCompletionResponse::new(
            buffered_string.to_owned(),
            Some(text.to_owned()),
            model_str.to_owned(),
        );
        if let Some(stats) = usage_stats {
            response = response.set_usage_statistics(stats);
        }
        sender.send(response).map_err(|e| {
            error!("Failed to send completion response: {}", e);
            LLMClientError::SendError(e)
        })
    }

    pub fn new(role: String, content: String) -> Self {
        Self {
            role,
            content: vec![AnthropicMessageContent::text(content, None)],
        }
    }
}

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AnthropicEvent {
    #[serde(rename = "message_start")]
    MessageStart {
        #[serde(rename = "message")]
        message: MessageData,
    },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        #[serde(rename = "index")]
        _index: u32,
        content_block: ContentBlockStart,
    },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        #[serde(rename = "index")]
        _index: u32,
        delta: ContentBlockDeltaType, // Using the new enum here
    },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop {
        #[serde(rename = "index")]
        _index: u32,
    },
    #[serde(rename = "message_delta")]
    MessageDelta {
        #[serde(rename = "delta")]
        _delta: MessageDeltaData,
        #[serde(rename = "usage")]
        usage: Usage,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
}

#[derive(Debug, Deserialize)]
struct MessageData {
    // id: String,
    // #[serde(rename = "type")]
    // message_type: String,
    // role: String,
    // content: Vec<String>,
    // model: String,
    // stop_reason: Option<String>,
    // stop_sequence: Option<String>,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ContentBlockStart {
    #[serde(rename = "tool_use")]
    InputToolUse { name: String, id: String },
    #[serde(rename = "text")]
    TextDelta { text: String },
}

#[derive(Debug, Deserialize)]
struct MessageDeltaData {
    // stop_reason: String,
    // stop_sequence: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
    cache_read_input_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ContentBlockDeltaType {
    #[serde(rename = "input_json_delta")]
    InputJsonDelta {
        #[serde(rename = "partial_json")]
        partial_json: String,
    },
    #[serde(rename = "text_delta")]
    TextDelta {
        text: String, // Reusing your `ContentBlockDelta.text` concept here
    },
}

#[derive(serde::Serialize, Debug, Clone)]
struct AnthropicRequest {
    system: Vec<AnthropicMessageContent>,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// This is going to be such a fucking nightmare later on...
    tools: Vec<serde_json::Value>,
    temperature: f32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    model: String,
}

impl AnthropicRequest {
    fn from_client_completion_request(
        completion_request: LLMClientCompletionRequest,
        model_str: String,
    ) -> Self {
        let messages = completion_request.messages();
        
        // Get system message content
        let system = messages
            .iter()
            .find(|m| m.role().is_system())
            .map(AnthropicMessage::collect_content)
            .unwrap_or_default();

        // Get tools from system message
        let tools = messages
            .iter()
            .find(|m| m.is_system_message())
            .map(|m| m.tools().into_iter().collect())
            .unwrap_or_default();

        // Convert user/assistant messages
        let messages = messages
            .into_iter()
            .filter(|m| m.role().is_user() || m.role().is_assistant())
            .map(|m| AnthropicMessage {
                role: m.role().to_string(),
                content: AnthropicMessage::collect_content(m),
            })
            .collect();

        Self {
            system,
            messages,
            temperature: completion_request.temperature(),
            max_tokens: completion_request.get_max_tokens().or(Some(8192)),
            tools,
            stream: true,
            model: model_str,
        }
    }

    fn from_client_string_request(
        completion_request: LLMClientCompletionStringRequest,
        model_str: String,
    ) -> Self {
        let temperature = completion_request.temperature();
        let max_tokens = completion_request.get_max_tokens();
        let messages = vec![AnthropicMessage::new(
            "user".to_owned(),
            completion_request.prompt().to_owned(),
        )];
        AnthropicRequest {
            system: vec![],
            messages,
            temperature,
            tools: vec![],
            stream: true,
            max_tokens,
            model: model_str,
        }
    }
}

pub struct AnthropicClient {
    client: reqwest_middleware::ClientWithMiddleware,
    base_url: String,
    chat_endpoint: String,
}

impl AnthropicClient {
    pub fn new() -> Self {
        Self {
            client: new_client(),
            base_url: "https://api.anthropic.com".to_owned(),
            chat_endpoint: "/v1/messages".to_owned(),
        }
    }

    pub fn new_with_custom_urls(base_url: String, chat_endpoint: String) -> Self {
        Self {
            client: new_client(),
            base_url,
            chat_endpoint,
        }
    }

    pub fn chat_endpoint(&self) -> String {
        format!("{}{}", &self.base_url, &self.chat_endpoint)
    }

    fn generate_api_bearer_key(
        &self,
        api_key: LLMProviderAPIKeys,
    ) -> Result<String, LLMClientError> {
        match api_key {
            LLMProviderAPIKeys::Anthropic(api_key) => Ok(api_key.api_key),
            _ => Err(LLMClientError::WrongAPIKeyType),
        }
    }

    fn get_model_string(&self, llm_type: &LLMType) -> Result<String, LLMClientError> {
        match llm_type {
            LLMType::ClaudeOpus => Ok("claude-3-opus-20240229".to_owned()),
            LLMType::ClaudeSonnet => Ok("claude-3-5-sonnet-20241022".to_owned()),
            LLMType::ClaudeHaiku => Ok("claude-3-haiku-20240307".to_owned()),
            LLMType::Custom(model) => Ok(model.to_owned()),
            _ => Err(LLMClientError::UnSupportedModel),
        }
    }

    /// We try to get the completion along with the tool which we are planning on using
    pub async fn stream_completion_with_tool(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionRequest,
        metadata: HashMap<String, String>,
        sender: UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<(String, Vec<(String, (String, String))>), LLMClientError> {
        let model_str = self.get_model_string(request.model())?;
        let messages = request.messages().to_vec();
        let anthropic_request = AnthropicRequest::from_client_completion_request(request, model_str.to_owned());

        let response_stream = self.send_request(&anthropic_request, api_key).await?;
        let mut event_source = response_stream.bytes_stream().eventsource();

        let mut response_text = String::new();
        let mut tool_uses = Vec::new();
        let mut active_tool = (None, None, String::new()); // (name, id, input_json)

        while let Some(Ok(event)) = event_source.next().await {
            if let Ok(event) = serde_json::from_str::<AnthropicEvent>(&event.data) {
                match event {
                    AnthropicEvent::ContentBlockStart { content_block, .. } => match content_block {
                        ContentBlockStart::InputToolUse { name, id } => {
                            active_tool = (Some(name.clone()), Some(id), String::new());
                            info!("anthropic::tool_use::{}", name);
                        }
                        ContentBlockStart::TextDelta { text } => {
                            response_text.push_str(&text);
                            AnthropicMessage::send_completion_response(
                                &response_text, &text, &model_str, &sender, None,
                            )?;
                        }
                    },
                    AnthropicEvent::ContentBlockDelta { delta, .. } => match delta {
                        ContentBlockDeltaType::TextDelta { text } => {
                            response_text.push_str(&text);
                            AnthropicMessage::send_completion_response(
                                &response_text, &text, &model_str, &sender, None,
                            )?;
                        }
                        ContentBlockDeltaType::InputJsonDelta { partial_json } => {
                            active_tool.2.push_str(&partial_json);
                        }
                    },
                    AnthropicEvent::ContentBlockStop { .. } => {
                        if let (Some(name), Some(id), input) = active_tool.clone() {
                            if !input.is_empty() {
                                tool_uses.push((name, (id, input)));
                            }
                        }
                        active_tool = (None, None, String::new());
                    }
                    AnthropicEvent::MessageStart { message } => {
                        debug!("anthropic::cache_hit::{:?}", message.usage.cache_read_input_tokens);
                    }
                    _ => {}
                }
            }
        }

        if tool_uses.is_empty() {
            info!("anthropic::tool_not_found");
        }

        let request_id = uuid::Uuid::new_v4().to_string();
        let parea_log_completion = PareaLogCompletion::new(
            messages.into_iter().map(|m| PareaLogMessage::new(
                m.role().to_string(),
                Self::format_message_content(&m),
            )).collect(),
            metadata.clone(),
            Self::format_completion_content(&response_text, &tool_uses),
            0.2,
            request_id.clone(),
            request_id.clone(),
            metadata.get("root_trace_id").cloned().unwrap_or_else(|| request_id.clone()),
            "ClaudeSonnet".to_owned(),
            "Anthropic".to_owned(),
            metadata.get("event_type").cloned().unwrap_or_else(|| "no_event_type".to_owned()),
        );
        let _ = PareaClient::new().log_completion(parea_log_completion).await;

        Ok((response_text, tool_uses))
    }
}

#[async_trait]
impl LLMClient for AnthropicClient {
    fn client(&self) -> &LLMProvider {
        &LLMProvider::Anthropic
    }

    async fn completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionRequest,
    ) -> Result<String, LLMClientError> {
        let (sender, _) = tokio::sync::mpsc::unbounded_channel();
        self.stream_completion(api_key, request, sender)
            .await
            .map(|answer| answer.answer_up_until_now().to_owned())
    }

    async fn stream_completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionRequest,
        sender: UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<LLMClientCompletionResponse, LLMClientError> {
        info!("anthropic::stream_completion");
        let endpoint = self.chat_endpoint();
        let model_str = self.get_model_string(request.model())?;
        let message_tokens = request
            .messages()
            .iter()
            .map(|message| message.content().len())
            .collect::<Vec<_>>();
        let mut message_tokens_count = 0;
        message_tokens.into_iter().for_each(|tokens| {
            message_tokens_count += tokens;
        });
        let anthropic_request =
            AnthropicRequest::from_client_completion_request(request, model_str.to_owned());

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let response_stream = self
            .client
            .post(endpoint)
            .header(
                "x-api-key".to_owned(),
                self.generate_api_bearer_key(api_key)?,
            )
            .header("anthropic-version".to_owned(), "2023-06-01".to_owned())
            .header("content-type".to_owned(), "application/json".to_owned())
            // anthropic-beta: prompt-caching-2024-07-31
            // enables prompt caching: https://arc.net/l/quote/qtlllqgf
            .header(
                "anthropic-beta".to_owned(),
                "prompt-caching-2024-07-31,max-tokens-3-5-sonnet-2024-07-15,computer-use-2024-10-22".to_owned(),
            )
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| {
                error!("sidecar.anthropic.error: {:?}", &e);
                e
            })?;

        // Check for 401 Unauthorized status
        if response_stream.status() == reqwest::StatusCode::UNAUTHORIZED {
            error!("Unauthorized access to Anthropic API");
            return Err(LLMClientError::UnauthorizedAccess);
        }

        let mut event_source = response_stream.bytes_stream().eventsource();

        let mut input_tokens = 0;
        let mut output_tokens = 0;
        let mut input_cached_tokens = 0;

        let mut buffered_string = "".to_owned();
        while let Some(Ok(event)) = event_source.next().await {
            // TODO: debugging this
            let event = serde_json::from_str::<AnthropicEvent>(&event.data);
            match event {
                Ok(AnthropicEvent::ContentBlockStart { content_block, .. }) => {
                    match content_block {
                        ContentBlockStart::InputToolUse { name, id: _id } => {
                            println!("anthropic::tool_use::{}", &name);
                        }
                        ContentBlockStart::TextDelta { text } => {
                            buffered_string = buffered_string + &text;
                            if let Err(e) = sender.send(
                                LLMClientCompletionResponse::new(
                                    buffered_string.to_owned(),
                                    Some(text),
                                    model_str.to_owned(),
                                )
                                .set_usage_statistics(
                                    LLMClientUsageStatistics::new()
                                        .set_input_tokens(input_tokens)
                                        .set_output_tokens(output_tokens),
                                ),
                            ) {
                                error!("Failed to send completion response: {}", e);
                                return Err(LLMClientError::SendError(e));
                            }
                        }
                    }
                }
                Ok(AnthropicEvent::ContentBlockDelta { delta, .. }) => match delta {
                    ContentBlockDeltaType::TextDelta { text } => {
                        buffered_string = buffered_string + &text;
                        let time_now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis();
                        let time_diff = time_now - current_time;
                        debug!(
                            event_name = "anthropic.buffered_string",
                            message_tokens_count = message_tokens_count,
                            generated_tokens_count = &buffered_string.len(),
                            time_taken = time_diff,
                        );
                        if let Err(e) = sender.send(
                            LLMClientCompletionResponse::new(
                                buffered_string.to_owned(),
                                Some(text),
                                model_str.to_owned(),
                            )
                            .set_usage_statistics(
                                LLMClientUsageStatistics::new()
                                    .set_input_tokens(input_tokens)
                                    .set_output_tokens(output_tokens)
                                    .set_cached_input_tokens(input_cached_tokens),
                            ),
                        ) {
                            error!("Failed to send completion response: {}", e);
                            return Err(LLMClientError::SendError(e));
                        }
                    }
                    ContentBlockDeltaType::InputJsonDelta { partial_json } => {
                        debug!("input_json_delta::{}", &partial_json);
                    }
                },
                Ok(AnthropicEvent::MessageStart { message }) => {
                    input_tokens = input_tokens + message.usage.input_tokens.unwrap_or_default();
                    output_tokens = output_tokens + message.usage.output_tokens.unwrap_or_default();
                    input_cached_tokens = input_cached_tokens
                        + message.usage.cache_read_input_tokens.unwrap_or_default();
                }
                Ok(AnthropicEvent::MessageDelta { _delta: _, usage }) => {
                    input_tokens = input_tokens + usage.input_tokens.unwrap_or_default();
                    output_tokens = output_tokens + usage.output_tokens.unwrap_or_default();
                    input_cached_tokens =
                        input_cached_tokens + usage.cache_read_input_tokens.unwrap_or_default();
                }
                Err(e) => {
                    println!("{:?}", e);
                    // break;
                }
                _ => {
                    debug!("Received anthropic event: {:?}", &event);
                }
            }
        }

        Ok(
            LLMClientCompletionResponse::new(buffered_string, None, model_str)
                .set_usage_statistics(
                    LLMClientUsageStatistics::new()
                        .set_input_tokens(input_tokens)
                        .set_output_tokens(output_tokens)
                        .set_cached_input_tokens(input_cached_tokens),
                ),
        )
    }

    async fn stream_prompt_completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionStringRequest,
        sender: UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<String, LLMClientError> {
        let model_str = self.get_model_string(request.model())?;
        let anthropic_request = AnthropicRequest::from_client_string_request(request, model_str.to_owned());

        let response_stream = self.send_request(&anthropic_request, api_key).await?;
        let mut event_source = response_stream.bytes_stream().eventsource();

        let mut buffered_string = String::new();
        let mut usage_stats = LLMClientUsageStatistics::new();

        while let Some(Ok(event)) = event_source.next().await {
            if let Ok(event) = serde_json::from_str::<AnthropicEvent>(&event.data) {
                match event {
                    AnthropicEvent::MessageStart { message } => {
                        if let Some(tokens) = message.usage.input_tokens {
                            usage_stats = usage_stats.set_input_tokens(tokens);
                        }
                        if let Some(tokens) = message.usage.output_tokens {
                            usage_stats = usage_stats.set_output_tokens(tokens);
                        }
                        if let Some(tokens) = message.usage.cache_read_input_tokens {
                            usage_stats = usage_stats.set_cached_input_tokens(tokens);
                        }
                    }
                    AnthropicEvent::MessageDelta { usage, .. } => {
                        if let Some(tokens) = usage.input_tokens {
                            usage_stats = usage_stats.set_input_tokens(tokens);
                        }
                        if let Some(tokens) = usage.output_tokens {
                            usage_stats = usage_stats.set_output_tokens(tokens);
                        }
                        if let Some(tokens) = usage.cache_read_input_tokens {
                            usage_stats = usage_stats.set_cached_input_tokens(tokens);
                        }
                    }
                    _ => {
                        AnthropicMessage::handle_event_content(
                            event,
                            &mut buffered_string,
                            &model_str,
                            &sender,
                            Some(usage_stats.clone()),
                        )?;
                    }
                }
            }
        }

        Ok(buffered_string)
    }
}