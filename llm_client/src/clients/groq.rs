use async_trait::async_trait;
use tracing::error;

use crate::provider::{LLMProvider, LLMProviderAPIKeys, OpenAICompatibleConfig};

use super::{
    openai_compatible::OpenAICompatibleClient,
    types::{
        LLMClient, LLMClientCompletionRequest, LLMClientCompletionResponse,
        LLMClientCompletionStringRequest, LLMClientError,
    },
};

pub struct GroqClient {
    openai_compatible_client: OpenAICompatibleClient,
}

impl GroqClient {
    pub fn new() -> Self {
        Self {
            openai_compatible_client: OpenAICompatibleClient::new(),
        }
    }

    fn api_base(&self) -> &str {
        "https://api.groq.com/openai/v1"
    }
}

#[async_trait]
impl LLMClient for GroqClient {
    fn client(&self) -> &LLMProvider {
        todo!()
    }

    async fn stream_completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionRequest,
        sender: tokio::sync::mpsc::UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<LLMClientCompletionResponse, LLMClientError> {
        match api_key {
            LLMProviderAPIKeys::GroqProvider(groq_api_key) => {
                let result = self
                    .openai_compatible_client
                    .stream_completion(
                        LLMProviderAPIKeys::OpenAICompatible(OpenAICompatibleConfig::new(
                            groq_api_key.api_key,
                            self.api_base().to_owned(),
                        )),
                        request,
                        sender,
                    )
                    .await;

                if let Err(ref e) = result {
                    error!("Failed to stream completion: {:?}", e);
                }
                result
            }
            _ => {
                error!("Wrong API key type provided for Groq client");
                Err(LLMClientError::WrongAPIKeyType)
            }
        }
    }

    async fn completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionRequest,
    ) -> Result<String, LLMClientError> {
        match api_key {
            LLMProviderAPIKeys::GroqProvider(groq_api_key) => {
                let result = self
                    .openai_compatible_client
                    .completion(
                        LLMProviderAPIKeys::OpenAICompatible(OpenAICompatibleConfig::new(
                            groq_api_key.api_key,
                            self.api_base().to_owned(),
                        )),
                        request,
                    )
                    .await;

                if let Err(ref e) = result {
                    error!("Failed to get completion: {:?}", e);
                }
                result
            }
            _ => {
                error!("Wrong API key type provided for Groq client");
                Err(LLMClientError::WrongAPIKeyType)
            }
        }
    }

    async fn stream_prompt_completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionStringRequest,
        sender: tokio::sync::mpsc::UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<String, LLMClientError> {
        match api_key {
            LLMProviderAPIKeys::GroqProvider(groq_api_key) => {
                let result = self
                    .openai_compatible_client
                    .stream_prompt_completion(
                        LLMProviderAPIKeys::OpenAICompatible(OpenAICompatibleConfig::new(
                            groq_api_key.api_key,
                            self.api_base().to_owned(),
                        )),
                        request,
                        sender,
                    )
                    .await;

                if let Err(ref e) = result {
                    error!("Failed to stream prompt completion: {:?}", e);
                }
                result
            }
            _ => {
                error!("Wrong API key type provided for Groq client");
                Err(LLMClientError::WrongAPIKeyType)
            }
        }
    }
}
