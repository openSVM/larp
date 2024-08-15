use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use async_trait::async_trait;
use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    provider::{LLMProvider, LLMProviderAPIKeys},
};

use crate::{
    agentic::{
        symbol::identifier::LLMProperties,
        tool::{
            code_symbol::{important::CodeSymbolImportantResponse, types::CodeSymbolError},
            errors::ToolError,
            file::types::FileImportantError,
            input::ToolInput,
            output::ToolOutput,
            r#type::Tool,
            search::{
                google_studio::GoogleStudioLLM,
                iterative::{IterativeSearchContext, IterativeSearchSystem},
                repository::Repository,
            },
        },
    },
    repomap::tag::TagIndex,
    tree_printer::tree::TreePrinter,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SearchType {
    Both,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BigSearchRequest {
    user_query: String,
    llm: LLMType,
    provider: LLMProvider,
    api_keys: LLMProviderAPIKeys,
    root_directory: Option<String>,
    root_request_id: String,
    search_type: SearchType,
}

impl BigSearchRequest {
    pub fn new(
        user_query: String,
        llm: LLMType,
        provider: LLMProvider,
        api_keys: LLMProviderAPIKeys,
        root_directory: Option<String>,
        root_request_id: String,
        search_type: SearchType,
    ) -> Self {
        Self {
            user_query,
            llm,
            provider,
            api_keys,
            root_directory,
            root_request_id,
            search_type,
        }
    }

    pub fn user_query(&self) -> &str {
        &self.user_query
    }

    pub fn llm(&self) -> &LLMType {
        &self.llm
    }

    pub fn provider(&self) -> &LLMProvider {
        &self.provider
    }

    pub fn api_keys(&self) -> &LLMProviderAPIKeys {
        &self.api_keys
    }

    pub fn root_directory(&self) -> Option<&str> {
        self.root_directory.as_deref()
    }

    pub fn root_request_id(&self) -> &str {
        &self.root_request_id
    }

    pub fn search_type(&self) -> &SearchType {
        &self.search_type
    }
}

#[async_trait]
pub trait BigSearch {
    async fn search(
        &self,
        input: BigSearchRequest,
    ) -> Result<CodeSymbolImportantResponse, CodeSymbolError>;
}

pub struct BigSearchBroker {
    llm_client: Arc<LLMBroker>,
    fail_over_llm: LLMProperties,
}

impl BigSearchBroker {
    pub fn new(llm_client: Arc<LLMBroker>, fail_over_llm: LLMProperties) -> Self {
        Self {
            llm_client,
            fail_over_llm,
        }
    }

    pub fn llm_client(&self) -> Arc<LLMBroker> {
        self.llm_client.clone()
    }

    pub fn fail_over_llm(&self) -> LLMProperties {
        self.fail_over_llm.clone()
    }

    fn validate_root_directory(&self, request: &BigSearchRequest) -> Result<String, ToolError> {
        request
            .root_directory()
            .ok_or_else(|| ToolError::BigSearchError("Root directory is required".to_string()))
            .map(|s| s.to_string())
    }

    async fn create_repository(&self, root_directory: &str) -> Result<Repository, ToolError> {
        let tree = TreePrinter::to_string_stacked(Path::new(root_directory)).unwrap_or_default();
        let tag_index = TagIndex::from_path(Path::new(root_directory)).await;

        Ok(Repository::new(
            tree,
            "outline".to_owned(),
            tag_index,
            PathBuf::from(root_directory),
        ))
    }

    fn create_search_system(
        &self,
        repository: Repository,
        request: &BigSearchRequest,
    ) -> Result<IterativeSearchSystem<GoogleStudioLLM>, ToolError> {
        let iterative_search_context =
            IterativeSearchContext::new(Vec::new(), request.user_query().to_owned(), String::new());

        let google_studio_llm_config = GoogleStudioLLM::new(
            request.root_directory().unwrap_or_default().to_owned(),
            self.llm_client(),
            request.root_request_id().to_owned(),
        );

        Ok(IterativeSearchSystem::new(
            iterative_search_context,
            repository,
            google_studio_llm_config,
        ))
    }
}

pub enum IterativeSearchSeed {
    Tree(String),
}

#[async_trait]
impl Tool for BigSearchBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let start = Instant::now();

        // Step 1: Parse input
        let parse_start = Instant::now();
        let request = input.big_search_query()?;
        println!("Step 1 - Parse input: {:?}", parse_start.elapsed());

        // Step 2: Validate root directory
        let validate_start = Instant::now();
        let root_directory = self.validate_root_directory(&request)?;
        println!(
            "Step 2 - Validate root directory: {:?}",
            validate_start.elapsed()
        );

        // Step 3: Create repository
        let repo_start = Instant::now();
        let repository = self.create_repository(&root_directory).await?;
        println!("Step 3 - Create repository: {:?}", repo_start.elapsed());

        // Step 4: Create search system
        let system_start = Instant::now();
        let mut system = self.create_search_system(repository, &request)?;
        println!(
            "Step 4 - Create search system: {:?}",
            system_start.elapsed()
        );

        let tree_string_start = Instant::now();

        let tree_string =
            TreePrinter::to_string_stacked(Path::new(&root_directory)).map_err(|_| {
                ToolError::FileImportantError(FileImportantError::PrintTreeError(root_directory))
            })?;

        let tree_seed = IterativeSearchSeed::Tree(tree_string);

        println!(
            "Step 4.5 - build tree string: {:?}",
            tree_string_start.elapsed()
        );

        // Step 5: Run search
        let search_start = Instant::now();
        let results = system
            .with_seed(tree_seed)
            .run()
            .await
            .map_err(|e| ToolError::IterativeSearchError(e))?;
        println!("Step 5 - Run search: {:?}", search_start.elapsed());

        let total_duration = start.elapsed();
        println!(
            "BigSearchBroker::invoke total duration: {:?}",
            total_duration
        );

        Ok(ToolOutput::BigSearch(results))
    }
}
