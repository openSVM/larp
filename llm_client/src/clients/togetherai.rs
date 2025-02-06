use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use logging::new_client;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error};

use crate::provider::LLMProviderAPIKeys;

use super::types::LLMClient;
use super::types::LLMClientCompletionRequest;
use super::types::LLMClientCompletionResponse;
use super::types::LLMClientCompletionStringRequest;
use super::types::LLMClientError;
use super::types::LLMType;

pub struct TogetherAIClient {
    pub client: reqwest_middleware::ClientWithMiddleware,
    pub base_url: String,
}

#[derive(serde::Serialize, Debug, Clone)]
struct TogetherAIRequestString {
    prompt: String,
    model: String,
    temperature: f32,
    stream_tokens: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
}

#[derive(serde::Serialize, Debug, Clone)]
struct TogetherAIMessage {
    role: String,
    content: String,
}

#[derive(serde::Serialize, Debug, Clone)]
struct TogetherAIRequestMessages {
    messages: Vec<TogetherAIMessage>,
    model: String,
    temperature: f32,
    stream_tokens: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct TogetherAIResponse {
    choices: Vec<Choice>,
    // id: String,
    // token: Token,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct TogetherAIRequestCompletion {
    choices: Vec<ChoiceCompletion>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct ChoiceCompletion {
    text: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Delta {
    content: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Choice {
    delta: Delta,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Token {
    id: i32,
    text: String,
    logprob: i32,
    special: bool,
}

impl TogetherAIRequestMessages {
    pub fn from_request(request: LLMClientCompletionRequest) -> Self {
        Self {
            messages: request
                .messages()
                .into_iter()
                .map(|message| TogetherAIMessage {
                    role: message.role().to_string(),
                    content: message.content().to_owned(),
                })
                .collect::<Vec<_>>(),
            model: TogetherAIClient::model_str(request.model()).expect("to be present"),
            temperature: request.temperature(),
            stream_tokens: true,
            frequency_penalty: request.frequency_penalty(),
            stop: request
                .stop_words()
                .map(|stop_words| stop_words.into_iter().map(|s| s.to_owned()).collect()),
        }
    }
}

impl TogetherAIRequestString {
    pub fn from_string_request(request: LLMClientCompletionStringRequest) -> Self {
        Self {
            prompt: request.prompt().to_owned(),
            model: TogetherAIClient::model_str(request.model()).expect("to be present"),
            temperature: request.temperature(),
            stream_tokens: true,
            frequency_penalty: request.frequency_penalty(),
            stop: request
                .stop_words()
                .map(|stop_words| stop_words.into_iter().map(|s| s.to_owned()).collect()),
            max_tokens: request.get_max_tokens(),
        }
    }
}

impl TogetherAIClient {
    pub fn new() -> Self {
        Self {
            client: new_client(),
            base_url: "https://api.together.xyz/v1".to_owned(),
        }
    }

    pub fn inference_endpoint(&self) -> String {
        format!("{}/completions", self.base_url)
    }

    pub fn completion_endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }

    pub fn model_str(model: &LLMType) -> Option<String> {
        match model {
            LLMType::Mixtral => Some("mistralai/Mixtral-8x7B-Instruct-v0.1".to_owned()),
            LLMType::MistralInstruct => Some("mistralai/Mistral-7B-Instruct-v0.1".to_owned()),
            LLMType::CodeLLama70BInstruct => Some("codellama/CodeLlama-70b-Instruct-hf".to_owned()),
            LLMType::Llama3_8bInstruct => Some("meta-llama/Meta-Llama-3-8B-Instruct".to_owned()),
            LLMType::DeepSeekCoder33BInstruct => {
                Some("deepseek-ai/deepseek-coder-33b-instruct".to_owned())
            }
            LLMType::Llama3_1_8bInstruct => {
                Some("accounts/fireworks/models/llama-v3p1-8b-instruct".to_owned())
            }
            LLMType::Custom(model) => Some(model.to_owned()),
            _ => None,
        }
    }

    fn generate_together_ai_bearer_key(
        &self,
        api_key: LLMProviderAPIKeys,
    ) -> Result<String, LLMClientError> {
        match api_key {
            LLMProviderAPIKeys::TogetherAI(api_key) => Ok(api_key.api_key),
            _ => Err(LLMClientError::WrongAPIKeyType),
        }
    }
}

#[async_trait]
impl LLMClient for TogetherAIClient {
    fn client(&self) -> &crate::provider::LLMProvider {
        &crate::provider::LLMProvider::TogetherAI
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
        let original_model_name = request.model().to_string();
        let model = TogetherAIClient::model_str(request.model());
        if model.is_none() {
            return Err(LLMClientError::FailedToGetResponse);
        }
        let together_ai_request = TogetherAIRequestString::from_string_request(request);
        debug!("sidecar.togetherai.request: {:?}", &together_ai_request);
        let mut response_stream = self
            .client
            .post(self.inference_endpoint())
            .bearer_auth(self.generate_together_ai_bearer_key(api_key)?.to_owned())
            .json(&together_ai_request)
            .send()
            .await?
            .bytes_stream()
            .eventsource();

        let mut buffered_string = "".to_owned();
        while let Some(event) = response_stream.next().await {
            match event {
                Ok(event) => {
                    if &event.data == "[DONE]" {
                        continue;
                    }
                    let value = serde_json::from_str::<TogetherAIRequestCompletion>(&event.data)?;
                    buffered_string = buffered_string + &value.choices[0].text;
                    debug!("sidecar.togetherai: {}", &buffered_string);
                    if let Err(e) = sender.send(LLMClientCompletionResponse::new(
                        buffered_string.to_owned(),
                        Some(value.choices[0].text.to_owned()),
                        original_model_name.to_owned(),
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
        let original_model_name = request.model().to_string();
        let model = TogetherAIClient::model_str(request.model());
        if model.is_none() {
            return Err(LLMClientError::FailedToGetResponse);
        }
        let together_ai_request = TogetherAIRequestMessages::from_request(request);
        let response = self
            .client
            .post(self.completion_endpoint())
            .bearer_auth(self.generate_together_ai_bearer_key(api_key)?.to_owned())
            .json(&together_ai_request)
            .send()
            .await?;

        // Check for unauthorized access
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            error!("Unauthorized access to Together AI API");
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
                    let value = serde_json::from_str::<TogetherAIResponse>(&event.data);
                    if let Ok(value) = value {
                        buffered_string = buffered_string + &value.choices[0].delta.content;
                        debug!("Stream content: {}", &buffered_string);
                        if let Err(e) = sender.send(LLMClientCompletionResponse::new(
                            buffered_string.to_owned(),
                            Some(value.choices[0].delta.content.to_owned()),
                            original_model_name.to_owned(),
                        )) {
                            error!("Failed to send completion response: {}", e);
                            return Err(LLMClientError::SendError(e));
                        }
                    }
                }
                Err(e) => {
                    error!("Stream error encountered: {:?}", e);
                }
            }
        }

        Ok(LLMClientCompletionResponse::new(
            buffered_string.to_owned(),
            None,
            original_model_name,
        ))
    }
}
