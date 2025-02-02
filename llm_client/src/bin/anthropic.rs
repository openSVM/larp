use llm_client::{
    clients::{
        anthropic::AnthropicClient,
        types::{LLMClient, LLMClientCompletionRequest, LLMClientMessage},
    },
    provider::AnthropicAPIKey,
};

#[tokio::main]
async fn main() {
    let api_key = "".to_owned();
    // put your api base over here
    // put the model name over here, not sure if this is really required for LMStudio?

    let openai_client = AnthropicClient::new();
    let api_key = llm_client::provider::LLMProviderAPIKeys::Anthropic(AnthropicAPIKey { api_key });
    let request = LLMClientCompletionRequest::new(
        llm_client::clients::types::LLMType::ClaudeSonnet,
        // llm_client::clients::types::LLMType::Custom("research-claude-wool".to_owned()),
        vec![
            LLMClientMessage::system("You are an expert software engineer".to_owned()),
            LLMClientMessage::user("Who are you?".to_owned()),
        ],
        //         vec![LLMClientMessage::system(
        //             "You are an expert code editor".to_owned(),
        //         ), LLMClientMessage::user(r#"Content for /repo/testing.rs
        // #[derive(Debug, serde::Serialize)]
        // pub enum FrameworkEvent {
        //     RepoMapGenerationStart(String),
        //     RepoMapGenerationFinished(String),
        //     LongContextSearchStart(String),
        //     LongContextSearchFinished(String),
        //     InitialSearchSymbols(InitialSearchSymbolEvent),
        //     OpenFile(OpenFileRequest),
        // }"#.to_owned()), LLMClientMessage::user(r#"I want you to edit the file in /repo/testing.rs the contents are shown below
        // ```rs
        // // FILEPATH: /repo/testing.rs
        // <code_in_selection>
        // #[derive(Debug, serde::Serialize)]
        // pub enum FrameworkEvent {
        //     RepoMapGenerationStart(String),
        //     RepoMapGenerationFinished(String),
        //     LongContextSearchStart(String),
        //     LongContextSearchFinished(String),
        //     InitialSearchSymbols(InitialSearchSymbolEvent),
        //     OpenFile(OpenFileRequest),
        // }
        // </code_in_selection>

        // <code_changes_outline>
        // #[derive(Debug, serde::Serialize)]
        // pub enum FrameworkEvent {
        //     /// Indicates the start of repository map generation process.
        //     /// The String parameter likely contains information about the repository or the process.
        //     RepoMapGenerationStart(String),

        //     /// Signals the completion of repository map generation.
        //     /// The String parameter might contain summary information or status of the generation process.
        //     RepoMapGenerationFinished(String),

        //     /// Marks the beginning of a long context search operation.
        //     /// The String parameter could contain search parameters or context information.
        //     LongContextSearchStart(String),

        //     /// Indicates the end of a long context search operation.
        //     /// The String parameter might contain search results or summary information.
        //     LongContextSearchFinished(String),

        //     /// Represents an event related to initial search symbols.
        //     /// Contains an InitialSearchSymbolEvent which likely holds detailed information about the symbols found.
        //     InitialSearchSymbols(InitialSearchSymbolEvent),

        //     /// Signifies a request to open a file.
        //     /// Contains an OpenFileRequest which probably includes the file path and other relevant details.
        //     OpenFile(OpenFileRequest),
        // }
        // </code_changes_outline>"#.to_owned()).insert_tool(serde_json::json!({
        //     "name": "str_replace_editor",
        //     "type": "text_editor_20241022",
        // }))],
        1.0,
        None,
    );
    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let response = openai_client
        .stream_completion(api_key, request, sender)
        .await;

    // wait for the magic to show up in your stdout
    dbg!(&response);
}
