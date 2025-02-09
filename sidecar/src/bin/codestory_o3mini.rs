use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage, LLMType},
    provider::{CodeStoryLLMTypes, LLMProvider, LLMProviderAPIKeys, CodestoryAccessToken},
};
use std::collections::HashMap;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create LLM broker
    let broker = LLMBroker::new().await?;

    // Create a CodeStory provider with O3MiniHigh model
    let provider = LLMProvider::CodeStory(CodeStoryLLMTypes {
        llm_type: Some(LLMType::O3MiniHigh),
    });

    // Example messages for the model
    let messages = vec![
        LLMClientMessage::system("You are a helpful AI assistant.".to_string()),
        LLMClientMessage::user("Hello! How can you help me today?".to_string()),
    ];

    // Create the completion request
    let request = LLMClientCompletionRequest::new(
        LLMType::O3MiniHigh,
        messages,
        0.7, // temperature
        None, // frequency penalty
    );

    // Example access token (replace with actual token in production)
    let api_key = LLMProviderAPIKeys::CodeStory(CodestoryAccessToken::new(
        "your_access_token".to_string(),
    ));

    // Create channel for receiving completion responses
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    // Stream completion in a separate task
    let completion_task = tokio::spawn(async move {
        let metadata = HashMap::new();
        broker
            .stream_completion(api_key, request, provider, metadata, sender)
            .await
    });

    // Process streaming responses
    while let Some(response) = receiver.recv().await {
        if let Some(delta) = response.delta() {
            print!("{}", delta);
        }
    }

    // Wait for completion task and get final result
    let result = completion_task.await??;
    println!("\nFinal response: {}", result.answer_up_until_now());

    Ok(())
}