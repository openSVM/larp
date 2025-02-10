use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use logging::new_client;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error};

use crate::provider::LLMProviderAPIKeys;

use super::types::{
    LLMClient, LLMClientCompletionRequest, LLMClientCompletionResponse,
    LLMClientCompletionStringRequest, LLMClientError, LLMType,
};

#[derive(serde::Serialize, Debug, Clone)]
struct FireworksAIRequestString {
    prompt: String,
    model: String,
    temperature: f32,
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<String>,
    stream: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct FireworksAIUsage {
    prompt_tokens: usize,
    total_tokens: usize,
    completion_tokens: usize,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct FireworksAIChatCompletion {
    choices: Vec<ChoiceCompletionChat>,
    usage: Option<FireworksAIUsage>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct FireworksAIRequestCompletion {
    choices: Vec<ChoiceCompletion>,
    usage: Option<FireworksAIUsage>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct ChoiceCompletionDelta {
    content: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct ChoiceCompletionChat {
    delta: ChoiceCompletionDelta,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct ChoiceCompletion {
    text: String,
}

#[derive(serde::Serialize, Debug, Clone)]
struct FireworksAIMessage {
    role: String,
    content: String,
}

#[derive(serde::Serialize, Debug, Clone)]
struct FireworksAIRequestChat {
    messages: Vec<FireworksAIMessage>,
    model: String,
    temperature: f32,
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<String>,
    stream: bool,
}

impl FireworksAIRequestChat {
    fn from_message(request: LLMClientCompletionRequest) -> FireworksAIRequestChat {
        FireworksAIRequestChat {
            messages: request
                .messages()
                .into_iter()
                .map(|message| FireworksAIMessage {
                    role: message.role().to_string(),
                    content: message.content().to_owned(),
                })
                .collect(),
            model: FireworksAIClient::model_str(request.model()).expect("to be present"),
            temperature: request.temperature(),
            max_tokens: request.get_max_tokens(),
            stop: request
                .stop_words()
                .map(|stop_words| stop_words.into_iter().map(|s| s.to_owned()).collect()),
            stream: true,
        }
    }
}

impl FireworksAIRequestString {
    fn from_string_message(
        string_request: LLMClientCompletionStringRequest,
    ) -> FireworksAIRequestString {
        FireworksAIRequestString {
            prompt: string_request.prompt().to_owned(),
            model: FireworksAIClient::model_str(string_request.model()).expect("to be present"),
            temperature: string_request.temperature(),
            max_tokens: string_request.get_max_tokens(),
            stop: string_request.stop_words().map(|stop_words| {
                stop_words
                    .into_iter()
                    .map(|s| s.to_owned())
                    .take(4)
                    .collect()
            }),
            stream: true,
        }
    }
}

pub struct FireworksAIClient {
    client: reqwest_middleware::ClientWithMiddleware,
    base_url: String,
}

impl FireworksAIClient {
    pub fn new() -> Self {
        let client = new_client();
        Self {
            client,
            base_url: "https://api.fireworks.ai/inference/v1".to_owned(),
        }
    }

    fn completion_endpoint(&self) -> String {
        format!("{}/completions", self.base_url)
    }

    fn chat_endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }

    fn model_str(model: &LLMType) -> Option<String> {
        match model {
            LLMType::CodeLlama13BInstruct => {
                Some("accounts/fireworks/models/llama-v2-13b-code".to_owned())
            }
            LLMType::Llama3_1_8bInstruct => {
                Some("accounts/fireworks/models/llama-v3p1-8b-instruct".to_owned())
            }
            LLMType::Llama3_1_70bInstruct => {
                Some("accounts/fireworks/models/llama-v3p1-70b-instruct".to_owned())
            }
            LLMType::DeepSeekR1 => {
                Some("accounts/fireworks/models/deepseek-r1".to_owned())
            }
            _ => None,
        }
    }

    fn generate_fireworks_ai_bearer_token(
        &self,
        api_key: LLMProviderAPIKeys,
    ) -> Result<String, LLMClientError> {
        match api_key {
            LLMProviderAPIKeys::FireworksAI(api_key) => Ok(api_key.api_key),
            _ => Err(LLMClientError::WrongAPIKeyType),
        }
    }
}

#[async_trait]
impl LLMClient for FireworksAIClient {
    fn client(&self) -> &crate::provider::LLMProvider {
        &crate::provider::LLMProvider::FireworksAI
    }

    async fn completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionRequest,
    ) -> Result<String, LLMClientError> {
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        self.stream_completion(api_key, request, sender)
            .await
            .map(|answer| answer.answer_up_until_now().to_owned())
    }

    async fn stream_prompt_completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionStringRequest,
        sender: UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<String, LLMClientError> {
        let original_model_str = request.model().to_string();
        let _ = FireworksAIClient::model_str(request.model())
            .ok_or(LLMClientError::UnSupportedModel)?;
        let bearer_token = self.generate_fireworks_ai_bearer_token(api_key)?;
        let request = FireworksAIRequestString::from_string_message(request);
        let mut response_stream = self
            .client
            .post(&self.completion_endpoint())
            .bearer_auth(bearer_token)
            .json(&request)
            .send()
            .await?
            .bytes_stream()
            .eventsource();

        let mut buffered_string = "".to_owned();
        while let Some(event) = response_stream.next().await {
            debug!("Stream event received: {:?}", &event);
            match event {
                Ok(event) => {
                    if &event.data == "[DONE]" {
                        continue;
                    }
                    let value = serde_json::from_str::<FireworksAIRequestCompletion>(&event.data)?;
                    if let Some(usage) = value.usage {
                        debug!("fireworksai::usage: {:?}", usage);
                    }
                    buffered_string.push_str(&value.choices[0].text);
                    if let Err(e) = sender.send(LLMClientCompletionResponse::new(
                        buffered_string.to_owned(),
                        Some(value.choices[0].text.to_owned()),
                        original_model_str.to_owned(),
                    )) {
                        error!("Failed to send completion response: {}", e);
                        return Err(LLMClientError::SendError(e));
                    }
                }
                Err(e) => {
                    error!("Stream error encountered: {:?}", e);
                }
            }
        }

        Ok(buffered_string)
    }

    async fn stream_completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionRequest,
        sender: UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<LLMClientCompletionResponse, LLMClientError> {
        let original_model_str = request.model().to_string();
        let _ = FireworksAIClient::model_str(request.model())
            .ok_or(LLMClientError::UnSupportedModel)?;
        let bearer_token = self.generate_fireworks_ai_bearer_token(api_key)?;
        let request = FireworksAIRequestChat::from_message(request);
        let response = self
            .client
            .post(&self.chat_endpoint())
            .bearer_auth(bearer_token)
            .json(&request)
            .send()
            .await?;

        // Check for unauthorized access
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            error!("Unauthorized access to Fireworks AI API");
            return Err(LLMClientError::UnauthorizedAccess);
        }

        let mut response_stream = response.bytes_stream().eventsource();

        let mut buffered_string = "".to_owned();
        while let Some(event) = response_stream.next().await {
            match event {
                Ok(event) => {
                    if &event.data == "[DONE]" {
                        continue;
                    }
                    let value = serde_json::from_str::<FireworksAIChatCompletion>(&event.data)?;
                    if let Some(usage) = value.usage {
                        debug!("fireworksai::stream_completion::usage: {:?}", &usage);
                    }
                    if let Some(content) = &value.choices[0].delta.content {
                        buffered_string.push_str(content);
                        if let Err(e) = sender.send(LLMClientCompletionResponse::new(
                            buffered_string.to_owned(),
                            Some(content.to_owned()),
                            original_model_str.to_owned(),
                        )) {
                            error!("Failed to send completion response: {}", e);
                            return Err(LLMClientError::SendError(e));
                        }
                    };
                }
                Err(e) => {
                    error!("Stream error encountered: {:?}", e);
                }
            }
        }

        Ok(LLMClientCompletionResponse::new(
            buffered_string,
            None,
            original_model_str,
        ))
    }
}