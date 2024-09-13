use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc};

use llm_client::broker::LLMBroker;

use crate::{
    agentic::symbol::identifier::LLMProperties, chunking::languages::TSLanguageParsing,
    inline_completion::symbols_tracker::SymbolTrackerInline,
};

use super::{
    code_edit::{
        filter_edit::FilterEditOperationBroker, find::FindCodeSectionsToEdit,
        models::broker::CodeEditBroker, search_and_replace::SearchAndReplaceEditing,
        test_correction::TestCorrection, types::CodeEditingTool,
    },
    code_symbol::{
        apply_outline_edit_to_range::ApplyOutlineEditsToRange, correctness::CodeCorrectnessBroker,
        error_fix::CodeSymbolErrorFixBroker, find_file_for_new_symbol::FindFileForNewSymbol,
        find_symbols_to_edit_in_context::FindSymbolsToEditInContext,
        followup::ClassSymbolFollowupBroker, important::CodeSymbolImportantBroker,
        initial_request_follow::CodeSymbolFollowInitialRequestBroker,
        new_location::CodeSymbolNewLocation, new_sub_symbol::NewSubSymbolRequired,
        planning_before_code_edit::PlanningBeforeCodeEdit, probe::ProbeEnoughOrDeeper,
        probe_question_for_symbol::ProbeQuestionForSymbol,
        probe_try_hard_answer::ProbeTryHardAnswer, repo_map_search::RepoMapSearchBroker,
        reranking_symbols_for_editing_context::ReRankingSnippetsForCodeEditingContext,
        scratch_pad::ScratchPadAgentBroker, should_edit::ShouldEditCodeSymbol,
    },
    editor::apply::EditorApply,
    errors::ToolError,
    file::file_finder::ImportantFilesFinderBroker,
    filtering::broker::CodeToEditFormatterBroker,
    git::diff_client::GitDiffClient,
    go_definition::go_definition::GoDefinitionEvaluatorBroker,
    grep::file::FindInFile,
    input::ToolInput,
    lsp::{
        diagnostics::LSPDiagnostics,
        get_outline_nodes::OutlineNodesUsingEditorClient,
        gotodefintion::LSPGoToDefinition,
        gotoimplementations::LSPGoToImplementation,
        gotoreferences::LSPGoToReferences,
        grep_symbol::GrepSymbolInCodebase,
        inlay_hints::InlayHints,
        open_file::LSPOpenFile,
        quick_fix::{LSPQuickFixClient, LSPQuickFixInvocationClient},
    },
    output::ToolOutput,
    r#type::{Tool, ToolType},
    ref_filter::ref_filter::ReferenceFilterBroker,
    rerank::base::ReRankBroker,
    search::big_search::BigSearchBroker,
    swe_bench::test_tool::SWEBenchTestTool,
};

pub struct ToolBrokerConfiguration {
    editor_agent: Option<LLMProperties>,
    apply_edits_directly: bool,
}

impl ToolBrokerConfiguration {
    pub fn new(editor_agent: Option<LLMProperties>, apply_edits_directly: bool) -> Self {
        Self {
            editor_agent,
            apply_edits_directly,
        }
    }
}

// TODO(skcd): We want to use a different serializer and deserializer for this
// since we are going to be storing an array of tools over here, we have to make
// sure that we do not store everything about the tool but a representation of it
pub struct ToolBroker {
    tools: HashMap<ToolType, Box<dyn Tool + Send + Sync>>,
}

impl ToolBroker {
    pub fn new(
        llm_client: Arc<LLMBroker>,
        code_edit_broker: Arc<CodeEditBroker>,
        symbol_tracking: Arc<SymbolTrackerInline>,
        language_broker: Arc<TSLanguageParsing>,
        tool_broker_config: ToolBrokerConfiguration,
        // Use this if the llm we were talking to times out or does not produce
        // outout which is coherent
        // we should have finer control over the fail-over llm but for now
        // a global setting like this is fine
        fail_over_llm: LLMProperties,
    ) -> Self {
        let mut tools: HashMap<ToolType, Box<dyn Tool + Send + Sync>> = Default::default();
        tools.insert(
            ToolType::CodeEditing,
            Box::new(
                CodeEditingTool::new(
                    llm_client.clone(),
                    code_edit_broker.clone(),
                    fail_over_llm.clone(),
                )
                .set_editor_config(tool_broker_config.editor_agent.clone()),
            ),
        );
        tools.insert(ToolType::LSPDiagnostics, Box::new(LSPDiagnostics::new()));
        tools.insert(
            ToolType::FindCodeSnippets,
            Box::new(FindCodeSectionsToEdit::new(
                symbol_tracking,
                language_broker,
                code_edit_broker.clone(),
                llm_client.clone(),
            )),
        );
        tools.insert(
            ToolType::ReRank,
            Box::new(ReRankBroker::new(llm_client.clone())),
        );
        tools.insert(
            ToolType::RequestImportantSymbols,
            Box::new(CodeSymbolImportantBroker::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::FindCodeSymbolsCodeBaseWide,
            Box::new(CodeSymbolImportantBroker::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::UtilityCodeSymbolSearch,
            Box::new(CodeSymbolImportantBroker::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::GoToDefinitions,
            Box::new(LSPGoToDefinition::new()),
        );
        tools.insert(ToolType::GoToReferences, Box::new(LSPGoToReferences::new()));
        tools.insert(ToolType::OpenFile, Box::new(LSPOpenFile::new()));
        tools.insert(ToolType::GrepInFile, Box::new(FindInFile::new()));
        tools.insert(
            ToolType::GoToImplementations,
            Box::new(LSPGoToImplementation::new()),
        );
        tools.insert(
            ToolType::FilterCodeSnippetsForEditing,
            Box::new(CodeToEditFormatterBroker::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::CodeCorrectnessActionSelection,
            Box::new(CodeCorrectnessBroker::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::CodeEditingForError,
            Box::new(CodeSymbolErrorFixBroker::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::FilterCodeSnippetsSingleSymbolForEditing,
            Box::new(CodeToEditFormatterBroker::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::EditorApplyEdits,
            Box::new(EditorApply::new(tool_broker_config.apply_edits_directly)),
        );
        tools.insert(ToolType::GetQuickFix, Box::new(LSPQuickFixClient::new()));
        tools.insert(
            ToolType::ApplyQuickFix,
            Box::new(LSPQuickFixInvocationClient::new()),
        );
        tools.insert(
            ToolType::ClassSymbolFollowup,
            Box::new(ClassSymbolFollowupBroker::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::ProbePossible,
            Box::new(CodeSymbolImportantBroker::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::ProbeQuestion,
            Box::new(CodeSymbolImportantBroker::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::ProbeSubSymbol,
            Box::new(CodeToEditFormatterBroker::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::ProbeFollowAlongSymbol,
            Box::new(CodeSymbolImportantBroker::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::ProbeSummarizeAnswer,
            Box::new(CodeSymbolImportantBroker::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::RepoMapSearch,
            Box::new(RepoMapSearchBroker::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::ImportantFilesFinder,
            Box::new(ImportantFilesFinderBroker::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        // todo
        tools.insert(
            ToolType::BigSearch,
            Box::new(BigSearchBroker::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::SWEBenchToolEndpoint,
            Box::new(SWEBenchTestTool::new()),
        );
        tools.insert(
            ToolType::TestCorrection,
            Box::new(TestCorrection::new(llm_client.clone())),
        );
        tools.insert(
            ToolType::CodeSymbolsToFollowInitialRequest,
            Box::new(CodeSymbolFollowInitialRequestBroker::new(
                llm_client.clone(),
            )),
        );
        tools.insert(
            ToolType::ProbeSubSymbolFiltering,
            Box::new(CodeToEditFormatterBroker::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::ProbeEnoughOrDeeper,
            Box::new(ProbeEnoughOrDeeper::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::ProbeCreateQuestionForSymbol,
            Box::new(ProbeQuestionForSymbol::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::PlanningBeforeCodeEdit,
            Box::new(PlanningBeforeCodeEdit::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::NewSubSymbolRequired,
            Box::new(NewSubSymbolRequired::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::ProbeTryHardAnswer,
            Box::new(ProbeTryHardAnswer::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::GrepSymbolInCodebase,
            Box::new(GrepSymbolInCodebase::new()),
        );
        tools.insert(
            ToolType::FindFileForNewSymbol,
            Box::new(FindFileForNewSymbol::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::FindSymbolsToEditInContext,
            Box::new(FindSymbolsToEditInContext::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::ReRankingCodeSnippetsForCodeEditingContext,
            Box::new(ReRankingSnippetsForCodeEditingContext::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::ApplyOutlineEditToRange,
            Box::new(ApplyOutlineEditsToRange::new(
                llm_client.clone(),
                fail_over_llm.clone(),
                // if we are not applying directly, then we are going to stream
                // the edits to the frontend
                !tool_broker_config.apply_edits_directly,
            )),
        );
        tools.insert(
            ToolType::FilterEditOperation,
            Box::new(FilterEditOperationBroker::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(ToolType::InLayHints, Box::new(InlayHints::new()));
        tools.insert(
            ToolType::CodeSymbolNewLocation,
            Box::new(CodeSymbolNewLocation::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::ShouldEditCode,
            Box::new(ShouldEditCodeSymbol::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::SearchAndReplaceEditing,
            Box::new(SearchAndReplaceEditing::new(
                llm_client.clone(),
                fail_over_llm.clone(),
                Arc::new(Box::new(LSPOpenFile::new())),
            )),
        );
        tools.insert(ToolType::GitDiff, Box::new(GitDiffClient::new()));
        tools.insert(
            ToolType::OutlineNodesUsingEditor,
            Box::new(OutlineNodesUsingEditorClient::new()),
        );
        tools.insert(
            ToolType::ReferencesFilter,
            Box::new(ReferenceFilterBroker::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        tools.insert(
            ToolType::ScratchPadAgent,
            Box::new(ScratchPadAgentBroker::new(llm_client.clone())),
        );
        tools.insert(
            ToolType::GoDefinitionEvaluator,
            Box::new(GoDefinitionEvaluatorBroker::new(llm_client.clone())),
        );
        // we also want to add the re-ranking tool here, so we invoke it freely
        Self { tools }
    }
}

#[async_trait]
impl Tool for ToolBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let tool_type = input.tool_type();
        if let Some(tool) = self.tools.get(&tool_type) {
            let result = tool.invoke(input).await;
            result
        } else {
            let result = Err(ToolError::MissingTool);
            result
        }
    }
}
