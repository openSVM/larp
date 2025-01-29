use llm_client::{
    clients::{
        groq::GroqClient,
        types::{LLMClient, LLMClientCompletionRequest, LLMClientMessage, LLMType},
    },
    provider::{GroqProviderAPIKey, LLMProviderAPIKeys},
};

#[tokio::main]
async fn main() {
    let system_message = r#"You are an expert software engineer who is an expert at applying edits made by another engineer to the code.
- The junior engineer was tasked with making changes to the code which is present in <code_in_selection> and they made higher level changes which are present in <code_changes_outline>
- You have to apply the changes made in <code_changes_outline> to <code_in_selection> and rewrite the code in <code_in_selection> after the changes have been made.
- Do not leave any placeholder comments or leave any logic out."#;
    let user_message = r#"<code_in_selection>
#[derive(Debug, serde::Serialize)]
pub enum FrameworkEvent {
    RepoMapGenerationStart(String),
    RepoMapGenerationFinished(String),
    LongContextSearchStart(String),
    LongContextSearchFinished(String),
    InitialSearchSymbols(InitialSearchSymbolEvent),
    OpenFile(OpenFileRequest),
}
</code_in_selection>

<code_changes_outline>
#[derive(Debug, serde::Serialize)]
pub enum FrameworkEvent {
    /// Indicates the start of repository map generation process.
    /// The String parameter likely contains information about the repository or the process.
    RepoMapGenerationStart(String),

    /// Signals the completion of repository map generation.
    /// The String parameter might contain summary information or status of the generation process.
    RepoMapGenerationFinished(String),

    /// Marks the beginning of a long context search operation.
    /// The String parameter could contain search parameters or context information.
    LongContextSearchStart(String),

    /// Indicates the end of a long context search operation.
    /// The String parameter might contain search results or summary information.
    LongContextSearchFinished(String),

    /// Represents an event related to initial search symbols.
    /// Contains an InitialSearchSymbolEvent which likely holds detailed information about the symbols found.
    InitialSearchSymbols(InitialSearchSymbolEvent),

    /// Signifies a request to open a file.
    /// Contains an OpenFileRequest which probably includes the file path and other relevant details.
    OpenFile(OpenFileRequest),
}
</code_changes_outline>

"#;
    // let gemini_llm_prperties = LLMProperties::new(
    //     LLMType::GeminiPro,
    //     LLMProvider::GoogleAIStudio,
    //     LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new(
    //         "".to_owned(),
    //     )),
    // );
    // let fireworks_ai = LLMProperties::new(
    //     LLMType::Llama3_1_8bInstruct,
    //     LLMProvider::Groq,
    //     LLMProviderAPIKeys::GroqProvider(GroqProviderAPIKey::new(
    //         "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".to_owned(),
    //     )),
    // );
    let few_shot_user_instruction = r#"<code_in_selection>
```py
def add_values(a, b):
    return a + b

def subtract(a, b):
    return a - b
```
</code_in_selection>

<code_changes_outline>
def add_values(a, b, logger):
    logger.info(a, b)
    # rest of the code

def subtract(a, b, logger):
    logger.info(a, b)
    # rest of the code
</code_changes_outline>"#;
    let few_shot_output = r#"<reply>
```py
def add_values(a, b, logger):
    logger.info(a, b)
    return a + b

def subtract(a, b, logger):
    logger.info(a, b)
    return a - b
```
</reply>"#;
    let llm_request = LLMClientCompletionRequest::new(
        LLMType::Llama3_1_70bInstruct,
        vec![
            LLMClientMessage::system(system_message.to_owned()),
            LLMClientMessage::user(few_shot_user_instruction.to_owned()),
            LLMClientMessage::assistant(few_shot_output.to_owned()),
            LLMClientMessage::user(user_message.to_owned()),
        ],
        0.0,
        None,
    );
    let client = GroqClient::new();
    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let start_instant = std::time::Instant::now();
    let response = client
        .stream_completion(
            LLMProviderAPIKeys::GroqProvider(GroqProviderAPIKey::new(
                "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".to_owned(),
            )),
            llm_request,
            sender,
        )
        .await;
    println!(
        "response {}:\n{}",
        start_instant.elapsed().as_millis(),
        response
            .expect("to work always")
            .answer_up_until_now()
            .to_owned()
    );
}
