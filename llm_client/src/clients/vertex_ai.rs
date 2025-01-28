use std::collections::HashMap;
use std::path::PathBuf;
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use google_cloud_auth::credentials::CredentialsFile;
use google_cloud_auth::token::{DefaultTokenSourceProvider, TokenSource};
use logging::new_client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use tracing::error;

use crate::provider::{LLMProvider, LLMProviderAPIKeys};
use super::types::{
    LLMClient, LLMClientCompletionRequest, LLMClientCompletionResponse,
    LLMClientCompletionStringRequest, LLMClientError, LLMClientMessage, LLMClientRole, LLMType,
};

pub struct VertexAIClient {
    client: reqwest_middleware::ClientWithMiddleware,
}

impl VertexAIClient {
    pub fn new() -> Self {
        Self {
            client: new_client(),
        }
    }

    fn get_api_endpoint(&self, project_id: &str, location: &str, model: &str) -> String {
        format!(
            "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:streamGenerateContent",
            location, project_id, location, model
        )
    }

    fn model(&self, model: &LLMType) -> Option<String> {
        match model {
            LLMType::GeminiPro => Some("gemini-1.0-pro".to_owned()),
            LLMType::Custom(llm_name) => Some(llm_name.to_owned()),
            _ => None,
        }
    }

    fn get_api_config(&self, api_key: &LLMProviderAPIKeys) -> Option<(String, String, String)> {
        match api_key {
            LLMProviderAPIKeys::VertexAI(config) => Some((
                config.project_id.clone(),
                config.location.clone(),
                config.credentials_path.clone(),
            )),
            _ => None,
        }
    }

    fn get_generation_config(&self, request: &LLMClientCompletionRequest) -> GenerationConfig {
        GenerationConfig {
            temperature: request.temperature(),
            max_output_tokens: request.get_max_tokens().unwrap_or(8192),
            candidate_count: 1,
            top_p: None,
            top_k: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    temperature: f32,
    top_p: Option<f32>,
    top_k: Option<u32>,
    max_output_tokens: usize,
    candidate_count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Content {
    role: String,
    parts: Vec<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VertexAIRequestBody {
    contents: Vec<Content>,
    generation_config: GenerationConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct VertexAIResponse {
    candidates: Vec<VertexAICandidate>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VertexAICandidate {
    content: Content,
}

#[async_trait]
impl LLMClient for VertexAIClient {
    fn client(&self) -> &LLMProvider {
        &LLMProvider::VertexAI
    }

    async fn stream_completion(
        &self,
        provider_api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionRequest,
        sender: UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<LLMClientCompletionResponse, LLMClientError> {
        let model = self.model(request.model());
        if model.is_none() {
            return Err(LLMClientError::UnSupportedModel);
        }
        let model = model.unwrap();

        let api_config = self.get_api_config(&provider_api_key);
        if api_config.is_none() {
            return Err(LLMClientError::WrongAPIKeyType);
        }
        let (project_id, location, credentials_path) = api_config.unwrap();

        // Convert messages to Vertex AI format
        let contents: Vec<Content> = request
            .messages()
            .iter()
            .map(|msg| Content {
                role: match msg.role() {
                    LLMClientRole::User => "user".to_string(),
                    LLMClientRole::Assistant => "assistant".to_string(),
                    LLMClientRole::System => "system".to_string(),
                    _ => "user".to_string(),
                },
                parts: vec![HashMap::from([("text".to_string(), msg.content().to_string())])],
            })
            .collect();

        let request_body = VertexAIRequestBody {
            contents,
            generation_config: self.get_generation_config(&request),
        };

        let endpoint = self.get_api_endpoint(&project_id, &location, &model);
        
        // Load credentials and get access token
        let credentials = CredentialsFile::new_from_file(PathBuf::from(credentials_path))
            .await
            .map_err(|_| LLMClientError::FailedToGetResponse)?;
            
        let token_source = DefaultTokenSourceProvider::new(credentials)
            .map_err(|_| LLMClientError::FailedToGetResponse)?;
            
        let token = token_source
            .token()
            .await
            .map_err(|_| LLMClientError::FailedToGetResponse)?;

        // Add authorization header with the token
        let response = self.client
            .post(&endpoint)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", token.access_token))
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await?;
            error!(
                "HTTP Error: {} {} - Response body: {}",
                status.as_u16(),
                status.as_str(),
                error_body
            );
            return Err(LLMClientError::FailedToGetResponse);
        }

        let mut buffered_string = String::new();
        let mut response_stream = response.bytes_stream().eventsource();
        
        while let Some(event) = response_stream.next().await {
            match event {
                Ok(event) => {
                    match serde_json::from_slice::<VertexAIResponse>(event.data.as_bytes()) {
                        Ok(parsed_event) => {
                            if let Some(text_part) = parsed_event.candidates[0].content.parts[0].get("text") {
                                buffered_string.push_str(text_part);
                                if let Err(e) = sender.send(LLMClientCompletionResponse::new(
                                    buffered_string.clone(),
                                    Some(text_part.to_owned()),
                                    model.to_owned(),
                                )) {
                                    error!("Failed to send completion response: {}", e);
                                    return Err(LLMClientError::SendError(e));
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse Vertex AI response: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Stream error encountered: {:?}", e);
                }
            }
        }

        Ok(LLMClientCompletionResponse::new(
            buffered_string,
            None,
            model,
        ))
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
        _api_key: LLMProviderAPIKeys,
        _request: LLMClientCompletionStringRequest,
        _sender: UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<String, LLMClientError> {
        Err(LLMClientError::GeminiProDoesNotSupportPromptCompletion)
    }
}