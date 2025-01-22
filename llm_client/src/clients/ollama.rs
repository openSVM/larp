//! Ollama client here so we can send requests to it

use async_trait::async_trait;
use logging::new_client;
use tokio::sync::mpsc::UnboundedSender;

use crate::provider::LLMProviderAPIKeys;

use super::types::LLMClient;
use super::types::LLMClientCompletionRequest;
use super::types::LLMClientCompletionResponse;
use super::types::LLMClientCompletionStringRequest;
use super::types::LLMClientError;
use super::types::LLMType;

pub struct OllamaClient {
    pub client: reqwest_middleware::ClientWithMiddleware,
    pub base_url: String,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct OllamaResponse {
    model: String,
    response: String,
}

impl LLMType {
    pub fn to_ollama_model(&self) -> Result<String, LLMClientError> {
        match self {
            LLMType::MistralInstruct => Ok("mistral".to_owned()),
            LLMType::Mixtral => Ok("mixtral".to_owned()),
            LLMType::CodeLLama70BInstruct => Ok("codellama70b".to_owned()),
            LLMType::DeepSeekCoder1_3BInstruct => Ok("deepseek-coder:1.3b-instruct".to_owned()),
            LLMType::DeepSeekCoder6BInstruct => Ok("deepseek-coder:6.7b-instruct".to_owned()),
            LLMType::Llama3_1_8bInstruct => Ok("llama3.1".to_owned()),
            LLMType::Custom(custom) => Ok(custom.to_owned()),
            _ => Err(LLMClientError::UnSupportedModel),
        }
    }
}

#[derive(serde::Serialize, Debug, Clone)]
struct OllamaClientOptions {
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<usize>,
}

#[derive(Debug, serde::Serialize)]
struct OllamaClientRequest {
    prompt: String,
    model: String,
    stream: bool,
    raw: bool,
    options: OllamaClientOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
}

impl OllamaClientRequest {
    pub fn from_request(request: LLMClientCompletionRequest) -> Result<Self, LLMClientError> {
        dbg!(request.model().to_ollama_model()?);
        Ok(Self {
            prompt: request
                .messages()
                .into_iter()
                .map(|message| message.content().to_owned())
                .collect::<Vec<_>>()
                .join("\n"),
            model: request.model().to_ollama_model()?,
            options: OllamaClientOptions {
                temperature: request.temperature(),
                num_predict: Some(1000),
            },
            stream: true,
            raw: true,
            frequency_penalty: request.frequency_penalty(),
        })
    }

    pub fn from_string_request(
        request: LLMClientCompletionStringRequest,
    ) -> Result<Self, LLMClientError> {
        Ok(Self {
            prompt: request.prompt().to_owned(),
            model: request.model().to_ollama_model()?,
            options: OllamaClientOptions {
                temperature: request.temperature(),
                num_predict: request.get_max_tokens(),
            },
            stream: true,
            raw: true,
            frequency_penalty: None,
        })
    }
}

impl OllamaClient {
    pub fn new() -> Self {
        // ollama always runs on the following url:
        // http://localhost:11434/
        Self {
            client: new_client(),
            base_url: "http://localhost:11434".to_owned(),
        }
    }

    pub fn generation_endpoint(&self) -> String {
        format!("{}/api/generate", self.base_url)
    }
}

#[async_trait]
impl LLMClient for OllamaClient {
    fn client(&self) -> &crate::provider::LLMProvider {
        &crate::provider::LLMProvider::Ollama
    }

    async fn stream_completion(
        &self,
        _api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionRequest,
        sender: tokio::sync::mpsc::UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<LLMClientCompletionResponse, LLMClientError> {
        let ollama_request = OllamaClientRequest::from_request(request)?;
        let mut response = self
            .client
            .post(self.generation_endpoint())
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| {
                dbg!(&e);
                e
            })?;

        let mut buffered_string = "".to_owned();
        while let Some(chunk) = response.chunk().await? {
            let value = serde_json::from_slice::<OllamaResponse>(chunk.to_vec().as_slice())?;
            buffered_string.push_str(&value.response);
            sender.send(LLMClientCompletionResponse::new(
                buffered_string.to_owned(),
                Some(value.response),
                value.model,
            ))?;
        }
        Ok(LLMClientCompletionResponse::new(buffered_string, None, ollama_request.model))
    }

    async fn completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionRequest,
    ) -> Result<String, LLMClientError> {
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        let result = self.stream_completion(api_key, request, sender).await?;
        Ok(result.answer_up_until_now().to_owned())
    }

    async fn stream_prompt_completion(
        &self,
        _api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionStringRequest,
        sender: UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<String, LLMClientError> {
        let prompt = request.prompt().to_owned();
        let ollama_request = OllamaClientRequest::from_string_request(request)?;
        let mut response = self
            .client
            .post(self.generation_endpoint())
            .json(&ollama_request)
            .send()
            .await?;

        let mut buffered_string = "".to_owned();
        while let Some(chunk) = response.chunk().await? {
            let value = serde_json::from_slice::<OllamaResponse>(chunk.to_vec().as_slice())?;
            buffered_string.push_str(&value.response);
            sender.send(LLMClientCompletionResponse::new(
                buffered_string.to_owned(),
                Some(value.response),
                value.model,
            ))?;
        }
        println!("request\n:{}", prompt);
        Ok(buffered_string)
    }
}
