use super::{
    code_edit::{
        filter_edit::FilterEditOperationRequest, find::FindCodeSelectionInput,
        search_and_replace::SearchAndReplaceEditingRequest,
        test_correction::TestOutputCorrectionRequest, types::CodeEdit,
    },
    code_symbol::{
        apply_outline_edit_to_range::ApplyOutlineEditsToRangeRequest,
        correctness::CodeCorrectnessRequest,
        error_fix::CodeEditingErrorRequest,
        find_file_for_new_symbol::FindFileForSymbolRequest,
        find_symbols_to_edit_in_context::FindSymbolsToEditInContextRequest,
        followup::ClassSymbolFollowupRequest,
        important::{
            CodeSymbolFollowAlongForProbing, CodeSymbolImportantRequest,
            CodeSymbolImportantWideSearch, CodeSymbolProbingSummarize,
            CodeSymbolToAskQuestionsRequest, CodeSymbolUtilityRequest,
        },
        initial_request_follow::CodeSymbolFollowInitialRequest,
        new_location::CodeSymbolNewLocationRequest,
        new_sub_symbol::NewSubSymbolRequiredRequest,
        planning_before_code_edit::PlanningBeforeCodeEditRequest,
        probe::ProbeEnoughOrDeeperRequest,
        probe_question_for_symbol::ProbeQuestionForSymbolRequest,
        probe_try_hard_answer::ProbeTryHardAnswerSymbolRequest,
        repo_map_search::RepoMapSearchQuery,
        reranking_symbols_for_editing_context::ReRankingSnippetsForCodeEditingRequest,
        scratch_pad::ScratchPadAgentInput,
        should_edit::ShouldEditCodeSymbolRequest,
    },
    editor::apply::EditorApplyRequest,
    errors::ToolError,
    file::file_finder::ImportantFilesFinderQuery,
    filtering::broker::{
        CodeToEditFilterRequest, CodeToEditSymbolRequest, CodeToProbeSubSymbolRequest,
    },
    git::diff_client::GitDiffClientRequest,
    grep::file::FindInFileRequest,
    kw_search::tool::KeywordSearchQuery,
    lsp::{
        diagnostics::LSPDiagnosticsInput,
        get_outline_nodes::OutlineNodesUsingEditorRequest,
        gotodefintion::GoToDefinitionRequest,
        gotoimplementations::GoToImplementationRequest,
        gotoreferences::GoToReferencesRequest,
        grep_symbol::LSPGrepSymbolInCodebaseRequest,
        inlay_hints::InlayHintsRequest,
        open_file::OpenFileRequest,
        quick_fix::{GetQuickFixRequest, LSPQuickFixInvocationRequest},
    },
    r#type::{Tool, ToolType},
    ref_filter::ref_filter::ReferenceFilterRequest,
    rerank::base::ReRankEntriesForBroker,
    search::big_search::BigSearchRequest,
    skill::skill::SkillSelectorRequest,
    swe_bench::test_tool::SWEBenchTestRequest,
};

#[derive(Debug, Clone)]
pub enum ToolInput {
    CodeEditing(CodeEdit),
    LSPDiagnostics(LSPDiagnosticsInput),
    FindCodeSnippets(FindCodeSelectionInput),
    ReRank(ReRankEntriesForBroker),
    CodeSymbolUtilitySearch(CodeSymbolUtilityRequest),
    RequestImportantSymbols(CodeSymbolImportantRequest),
    RequestImportantSymbolsCodeWide(CodeSymbolImportantWideSearch),
    GoToDefinition(GoToDefinitionRequest),
    GoToReference(GoToReferencesRequest),
    OpenFile(OpenFileRequest),
    GrepSingleFile(FindInFileRequest),
    SymbolImplementations(GoToImplementationRequest),
    FilterCodeSnippetsForEditing(CodeToEditFilterRequest),
    FilterCodeSnippetsForEditingSingleSymbols(CodeToEditSymbolRequest),
    EditorApplyChange(EditorApplyRequest),
    QuickFixRequest(GetQuickFixRequest),
    QuickFixInvocationRequest(LSPQuickFixInvocationRequest),
    CodeCorrectnessAction(CodeCorrectnessRequest),
    CodeEditingError(CodeEditingErrorRequest),
    ClassSymbolFollowup(ClassSymbolFollowupRequest),
    // probe request
    ProbeCreateQuestionForSymbol(ProbeQuestionForSymbolRequest),
    ProbeEnoughOrDeeper(ProbeEnoughOrDeeperRequest),
    ProbeFilterSnippetsSingleSymbol(CodeToProbeSubSymbolRequest),
    ProbeSubSymbol(CodeToEditFilterRequest),
    ProbePossibleRequest(CodeSymbolToAskQuestionsRequest),
    ProbeQuestionAskRequest(CodeSymbolToAskQuestionsRequest),
    ProbeFollowAlongSymbol(CodeSymbolFollowAlongForProbing),
    ProbeSummarizeAnswerRequest(CodeSymbolProbingSummarize),
    ProbeTryHardAnswerRequest(ProbeTryHardAnswerSymbolRequest),
    // repo map query
    RepoMapSearch(RepoMapSearchQuery),
    // important files query
    ImportantFilesFinder(ImportantFilesFinderQuery),
    // SWE Bench tooling
    SWEBenchTest(SWEBenchTestRequest),
    // Test output correction
    TestOutputCorrection(TestOutputCorrectionRequest),
    // Code symbol follow initial request
    CodeSymbolFollowInitialRequest(CodeSymbolFollowInitialRequest),
    // Plan before code editing
    PlanningBeforeCodeEdit(PlanningBeforeCodeEditRequest),
    // New symbols required for code editing
    NewSubSymbolForCodeEditing(NewSubSymbolRequiredRequest),
    // Find the symbol in the codebase which we want to select, this only
    // takes a string as input
    GrepSymbolInCodebase(LSPGrepSymbolInCodebaseRequest),
    // Find file location for the new symbol
    FindFileForNewSymbol(FindFileForSymbolRequest),
    // Find symbol to edit in user context
    FindSymbolsToEditInContext(FindSymbolsToEditInContextRequest),
    // ReRanking outline nodes for code editing context
    ReRankingCodeSnippetsForEditing(ReRankingSnippetsForCodeEditingRequest),
    // Apply the generated code outline to the range we are interested in
    ApplyOutlineEditToRange(ApplyOutlineEditsToRangeRequest),
    // Big search
    BigSearch(BigSearchRequest),
    // checks if the edit operation needs to be performed or is an extra
    FilterEditOperation(FilterEditOperationRequest),
    // Keyword search
    KeywordSearch(KeywordSearchQuery),
    // inlay hints from the lsp/editor
    InlayHints(InlayHintsRequest),
    CodeSymbolNewLocation(CodeSymbolNewLocationRequest),
    // should edit the code symbol
    ShouldEditCode(ShouldEditCodeSymbolRequest),
    // search and replace blocks
    SearchAndReplaceEditing(SearchAndReplaceEditingRequest),
    // git diff request
    GitDiff(GitDiffClientRequest),
    OutlineNodesUsingEditor(OutlineNodesUsingEditorRequest),
    // filters references based on user query
    ReferencesFilter(ReferenceFilterRequest),
    // Scratch pad agent input request
    ScratchPadInput(ScratchPadAgentInput),
    // Skillz
    SkillInput(SkillSelectorRequest),
}

impl ToolInput {
    pub fn tool_type(&self) -> ToolType {
        match self {
            ToolInput::CodeEditing(_) => ToolType::CodeEditing,
            ToolInput::LSPDiagnostics(_) => ToolType::LSPDiagnostics,
            ToolInput::FindCodeSnippets(_) => ToolType::FindCodeSnippets,
            ToolInput::ReRank(_) => ToolType::ReRank,
            ToolInput::RequestImportantSymbols(_) => ToolType::RequestImportantSymbols,
            ToolInput::RequestImportantSymbolsCodeWide(_) => ToolType::FindCodeSymbolsCodeBaseWide,
            ToolInput::GoToDefinition(_) => ToolType::GoToDefinitions,
            ToolInput::GoToReference(_) => ToolType::GoToReferences,
            ToolInput::OpenFile(_) => ToolType::OpenFile,
            ToolInput::GrepSingleFile(_) => ToolType::GrepInFile,
            ToolInput::SymbolImplementations(_) => ToolType::GoToImplementations,
            ToolInput::FilterCodeSnippetsForEditing(_) => ToolType::FilterCodeSnippetsForEditing,
            ToolInput::FilterCodeSnippetsForEditingSingleSymbols(_) => {
                ToolType::FilterCodeSnippetsSingleSymbolForEditing
            }
            ToolInput::EditorApplyChange(_) => ToolType::EditorApplyEdits,
            ToolInput::CodeSymbolUtilitySearch(_) => ToolType::UtilityCodeSymbolSearch,
            ToolInput::QuickFixRequest(_) => ToolType::GetQuickFix,
            ToolInput::QuickFixInvocationRequest(_) => ToolType::ApplyQuickFix,
            ToolInput::CodeCorrectnessAction(_) => ToolType::CodeCorrectnessActionSelection,
            ToolInput::CodeEditingError(_) => ToolType::CodeEditingForError,
            ToolInput::ClassSymbolFollowup(_) => ToolType::ClassSymbolFollowup,
            ToolInput::ProbePossibleRequest(_) => ToolType::ProbePossible,
            ToolInput::ProbeQuestionAskRequest(_) => ToolType::ProbeQuestion,
            ToolInput::ProbeSubSymbol(_) => ToolType::ProbeSubSymbol,
            ToolInput::ProbeFollowAlongSymbol(_) => ToolType::ProbeFollowAlongSymbol,
            ToolInput::ProbeSummarizeAnswerRequest(_) => ToolType::ProbeSummarizeAnswer,
            ToolInput::RepoMapSearch(_) => ToolType::RepoMapSearch,
            ToolInput::ImportantFilesFinder(_) => ToolType::ImportantFilesFinder,
            ToolInput::SWEBenchTest(_) => ToolType::SWEBenchToolEndpoint,
            ToolInput::TestOutputCorrection(_) => ToolType::TestCorrection,
            ToolInput::CodeSymbolFollowInitialRequest(_) => {
                ToolType::CodeSymbolsToFollowInitialRequest
            }
            ToolInput::ProbeFilterSnippetsSingleSymbol(_) => ToolType::ProbeSubSymbolFiltering,
            ToolInput::ProbeEnoughOrDeeper(_) => ToolType::ProbeEnoughOrDeeper,
            ToolInput::ProbeCreateQuestionForSymbol(_) => ToolType::ProbeCreateQuestionForSymbol,
            ToolInput::PlanningBeforeCodeEdit(_) => ToolType::PlanningBeforeCodeEdit,
            ToolInput::NewSubSymbolForCodeEditing(_) => ToolType::NewSubSymbolRequired,
            ToolInput::ProbeTryHardAnswerRequest(_) => ToolType::ProbeTryHardAnswer,
            ToolInput::GrepSymbolInCodebase(_) => ToolType::GrepSymbolInCodebase,
            ToolInput::FindFileForNewSymbol(_) => ToolType::FindFileForNewSymbol,
            ToolInput::FindSymbolsToEditInContext(_) => ToolType::FindSymbolsToEditInContext,
            ToolInput::ReRankingCodeSnippetsForEditing(_) => {
                ToolType::ReRankingCodeSnippetsForCodeEditingContext
            }
            ToolInput::ApplyOutlineEditToRange(_) => ToolType::ApplyOutlineEditToRange,
            ToolInput::BigSearch(_) => ToolType::BigSearch,
            ToolInput::FilterEditOperation(_) => ToolType::FilterEditOperation,
            ToolInput::KeywordSearch(_) => ToolType::KeywordSearch,
            ToolInput::InlayHints(_) => ToolType::InLayHints,
            ToolInput::CodeSymbolNewLocation(_) => ToolType::CodeSymbolNewLocation,
            ToolInput::ShouldEditCode(_) => ToolType::ShouldEditCode,
            ToolInput::SearchAndReplaceEditing(_) => ToolType::SearchAndReplaceEditing,
            ToolInput::GitDiff(_) => ToolType::GitDiff,
            ToolInput::OutlineNodesUsingEditor(_) => ToolType::OutlineNodesUsingEditor,
            ToolInput::ReferencesFilter(_) => ToolType::ReferencesFilter,
            ToolInput::ScratchPadInput(_) => ToolType::ScratchPadAgent,
            ToolInput::SkillInput(_) => ToolType::SkillSelector,
        }
    }

    pub fn should_scratch_pad_input(self) -> Result<ScratchPadAgentInput, ToolError> {
        if let ToolInput::ScratchPadInput(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ScratchPadAgent))
        }
    }

    pub fn should_outline_nodes_using_editor(
        self,
    ) -> Result<OutlineNodesUsingEditorRequest, ToolError> {
        if let ToolInput::OutlineNodesUsingEditor(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::OutlineNodesUsingEditor))
        }
    }

    pub fn should_git_diff(self) -> Result<GitDiffClientRequest, ToolError> {
        if let ToolInput::GitDiff(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::GitDiff))
        }
    }

    pub fn should_search_and_replace_editing(
        self,
    ) -> Result<SearchAndReplaceEditingRequest, ToolError> {
        if let ToolInput::SearchAndReplaceEditing(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::SearchAndReplaceEditing))
        }
    }

    pub fn should_edit_code(self) -> Result<ShouldEditCodeSymbolRequest, ToolError> {
        if let ToolInput::ShouldEditCode(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ShouldEditCode))
        }
    }

    pub fn code_symbol_new_location(self) -> Result<CodeSymbolNewLocationRequest, ToolError> {
        if let ToolInput::CodeSymbolNewLocation(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::CodeSymbolNewLocation))
        }
    }

    pub fn inlay_hints_request(self) -> Result<InlayHintsRequest, ToolError> {
        if let ToolInput::InlayHints(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::InLayHints))
        }
    }

    pub fn filter_edit_operation_request(self) -> Result<FilterEditOperationRequest, ToolError> {
        if let ToolInput::FilterEditOperation(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::FilterEditOperation))
        }
    }

    pub fn filter_references_request(self) -> Result<ReferenceFilterRequest, ToolError> {
        if let ToolInput::ReferencesFilter(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ReferencesFilter))
        }
    }

    pub fn apply_outline_edits_to_range(
        self,
    ) -> Result<ApplyOutlineEditsToRangeRequest, ToolError> {
        if let ToolInput::ApplyOutlineEditToRange(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ApplyOutlineEditToRange))
        }
    }

    pub fn reranking_code_snippets_for_editing_context(
        self,
    ) -> Result<ReRankingSnippetsForCodeEditingRequest, ToolError> {
        if let ToolInput::ReRankingCodeSnippetsForEditing(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(
                ToolType::ReRankingCodeSnippetsForCodeEditingContext,
            ))
        }
    }

    pub fn find_symbols_to_edit_in_context(
        self,
    ) -> Result<FindSymbolsToEditInContextRequest, ToolError> {
        if let ToolInput::FindSymbolsToEditInContext(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(
                ToolType::FindSymbolsToEditInContext,
            ))
        }
    }

    pub fn find_file_for_new_symbol(self) -> Result<FindFileForSymbolRequest, ToolError> {
        if let ToolInput::FindFileForNewSymbol(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::FindFileForNewSymbol))
        }
    }

    pub fn grep_symbol_in_codebase(self) -> Result<LSPGrepSymbolInCodebaseRequest, ToolError> {
        if let ToolInput::GrepSymbolInCodebase(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::GrepSymbolInCodebase))
        }
    }

    pub fn get_probe_try_hard_answer_request(
        self,
    ) -> Result<ProbeTryHardAnswerSymbolRequest, ToolError> {
        if let ToolInput::ProbeTryHardAnswerRequest(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ProbeTryHardAnswer))
        }
    }

    pub fn probe_try_hard_answer(request: ProbeTryHardAnswerSymbolRequest) -> Self {
        ToolInput::ProbeTryHardAnswerRequest(request)
    }

    pub fn get_new_sub_symbol_for_code_editing(
        self,
    ) -> Result<NewSubSymbolRequiredRequest, ToolError> {
        if let ToolInput::NewSubSymbolForCodeEditing(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::NewSubSymbolRequired))
        }
    }

    pub fn probe_create_question_for_symbol(request: ProbeQuestionForSymbolRequest) -> Self {
        ToolInput::ProbeCreateQuestionForSymbol(request)
    }

    pub fn get_probe_create_question_for_symbol(
        self,
    ) -> Result<ProbeQuestionForSymbolRequest, ToolError> {
        if let ToolInput::ProbeCreateQuestionForSymbol(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(
                ToolType::ProbeCreateQuestionForSymbol,
            ))
        }
    }

    pub fn probe_enough_or_deeper(request: ProbeEnoughOrDeeperRequest) -> Self {
        ToolInput::ProbeEnoughOrDeeper(request)
    }

    pub fn get_probe_enough_or_deeper(self) -> Result<ProbeEnoughOrDeeperRequest, ToolError> {
        if let ToolInput::ProbeEnoughOrDeeper(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ProbeEnoughOrDeeper))
        }
    }

    pub fn probe_filter_snippets_single_symbol(request: CodeToProbeSubSymbolRequest) -> Self {
        ToolInput::ProbeFilterSnippetsSingleSymbol(request)
    }

    pub fn is_probe_filter_snippets_single_symbol(&self) -> bool {
        if let ToolInput::ProbeFilterSnippetsSingleSymbol(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_code_symbol_follow_initial_request(
        self,
    ) -> Result<CodeSymbolFollowInitialRequest, ToolError> {
        if let ToolInput::CodeSymbolFollowInitialRequest(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(
                ToolType::CodeSymbolsToFollowInitialRequest,
            ))
        }
    }

    pub fn is_test_output(self) -> Result<TestOutputCorrectionRequest, ToolError> {
        if let ToolInput::TestOutputCorrection(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::TestCorrection))
        }
    }

    pub fn is_probe_subsymbol(&self) -> bool {
        if let ToolInput::ProbeSubSymbol(_) = self {
            true
        } else {
            false
        }
    }

    pub fn swe_bench_test(self) -> Result<SWEBenchTestRequest, ToolError> {
        if let ToolInput::SWEBenchTest(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::SWEBenchToolEndpoint))
        }
    }

    pub fn repo_map_search_query(self) -> Result<RepoMapSearchQuery, ToolError> {
        if let ToolInput::RepoMapSearch(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::RepoMapSearch))
        }
    }

    pub fn important_files_finder_query(self) -> Result<ImportantFilesFinderQuery, ToolError> {
        if let ToolInput::ImportantFilesFinder(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ImportantFilesFinder))
        }
    }

    pub fn keyword_search_query(self) -> Result<KeywordSearchQuery, ToolError> {
        if let ToolInput::KeywordSearch(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::KeywordSearch))
        }
    }

    pub fn big_search_query(self) -> Result<BigSearchRequest, ToolError> {
        if let ToolInput::BigSearch(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::BigSearch))
        }
    }

    pub fn probe_sub_symbol_filtering(self) -> Result<CodeToProbeSubSymbolRequest, ToolError> {
        if let ToolInput::ProbeFilterSnippetsSingleSymbol(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ProbeSubSymbolFiltering))
        }
    }

    pub fn probe_subsymbol(self) -> Result<CodeToEditFilterRequest, ToolError> {
        if let ToolInput::ProbeSubSymbol(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ProbeSubSymbol))
        }
    }

    pub fn probe_possible_request(self) -> Result<CodeSymbolToAskQuestionsRequest, ToolError> {
        if let ToolInput::ProbePossibleRequest(output) = self {
            Ok(output)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ProbePossible))
        }
    }

    pub fn probe_question_request(self) -> Result<CodeSymbolToAskQuestionsRequest, ToolError> {
        if let ToolInput::ProbeQuestionAskRequest(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ProbeQuestion))
        }
    }

    pub fn probe_follow_along_symbol(self) -> Result<CodeSymbolFollowAlongForProbing, ToolError> {
        if let ToolInput::ProbeFollowAlongSymbol(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ProbeFollowAlongSymbol))
        }
    }

    pub fn probe_summarization_request(self) -> Result<CodeSymbolProbingSummarize, ToolError> {
        if let ToolInput::ProbeSummarizeAnswerRequest(response) = self {
            Ok(response)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ProbeSummarizeAnswer))
        }
    }

    pub fn is_probe_summarization_request(&self) -> bool {
        if let ToolInput::ProbeSummarizeAnswerRequest(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_repo_map_search(&self) -> bool {
        if let ToolInput::RepoMapSearch(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_probe_follow_along_symbol_request(&self) -> bool {
        if let ToolInput::ProbeFollowAlongSymbol(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_probe_possible_request(&self) -> bool {
        if let ToolInput::ProbePossibleRequest(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_probe_question(&self) -> bool {
        if let ToolInput::ProbeQuestionAskRequest(_) = self {
            true
        } else {
            false
        }
    }

    pub fn code_editing_error(self) -> Result<CodeEditingErrorRequest, ToolError> {
        if let ToolInput::CodeEditingError(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::CodeEditingForError))
        }
    }

    pub fn code_correctness_action(self) -> Result<CodeCorrectnessRequest, ToolError> {
        if let ToolInput::CodeCorrectnessAction(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(
                ToolType::CodeCorrectnessActionSelection,
            ))
        }
    }

    pub fn quick_fix_invocation_request(self) -> Result<LSPQuickFixInvocationRequest, ToolError> {
        if let ToolInput::QuickFixInvocationRequest(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::GetQuickFix))
        }
    }

    pub fn quick_fix_request(self) -> Result<GetQuickFixRequest, ToolError> {
        if let ToolInput::QuickFixRequest(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ApplyQuickFix))
        }
    }

    pub fn editor_apply_changes(self) -> Result<EditorApplyRequest, ToolError> {
        if let ToolInput::EditorApplyChange(editor_apply_request) = self {
            Ok(editor_apply_request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::EditorApplyEdits))
        }
    }

    pub fn symbol_implementations(self) -> Result<GoToImplementationRequest, ToolError> {
        if let ToolInput::SymbolImplementations(symbol_implementations) = self {
            Ok(symbol_implementations)
        } else {
            Err(ToolError::WrongToolInput(ToolType::GoToImplementations))
        }
    }

    pub fn reference_request(self) -> Result<GoToReferencesRequest, ToolError> {
        if let ToolInput::GoToReference(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::GoToReferences))
        }
    }

    pub fn class_symbol_followup(self) -> Result<ClassSymbolFollowupRequest, ToolError> {
        if let ToolInput::ClassSymbolFollowup(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ClassSymbolFollowup))
        }
    }

    pub fn grep_single_file(self) -> Result<FindInFileRequest, ToolError> {
        if let ToolInput::GrepSingleFile(grep_single_file) = self {
            Ok(grep_single_file)
        } else {
            Err(ToolError::WrongToolInput(ToolType::GrepInFile))
        }
    }

    pub fn is_file_open(self) -> Result<OpenFileRequest, ToolError> {
        if let ToolInput::OpenFile(open_file) = self {
            Ok(open_file)
        } else {
            Err(ToolError::WrongToolInput(ToolType::OpenFile))
        }
    }

    pub fn is_go_to_definition(self) -> Result<GoToDefinitionRequest, ToolError> {
        if let ToolInput::GoToDefinition(definition_request) = self {
            Ok(definition_request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::GoToDefinitions))
        }
    }

    pub fn is_code_edit(self) -> Result<CodeEdit, ToolError> {
        if let ToolInput::CodeEditing(code_edit) = self {
            Ok(code_edit)
        } else {
            Err(ToolError::WrongToolInput(ToolType::CodeEditing))
        }
    }

    pub fn is_lsp_diagnostics(self) -> Result<LSPDiagnosticsInput, ToolError> {
        if let ToolInput::LSPDiagnostics(lsp_diagnostics) = self {
            Ok(lsp_diagnostics)
        } else {
            Err(ToolError::WrongToolInput(ToolType::LSPDiagnostics))
        }
    }

    pub fn is_code_find(self) -> Result<FindCodeSelectionInput, ToolError> {
        if let ToolInput::FindCodeSnippets(find_code_snippets) = self {
            Ok(find_code_snippets)
        } else {
            Err(ToolError::WrongToolInput(ToolType::FindCodeSnippets))
        }
    }

    pub fn is_rerank(self) -> Result<ReRankEntriesForBroker, ToolError> {
        if let ToolInput::ReRank(rerank) = self {
            Ok(rerank)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ReRank))
        }
    }

    pub fn is_utility_code_search(&self) -> bool {
        if let ToolInput::CodeSymbolUtilitySearch(_) = self {
            true
        } else {
            false
        }
    }

    pub fn utility_code_search(self) -> Result<CodeSymbolUtilityRequest, ToolError> {
        if let ToolInput::CodeSymbolUtilitySearch(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::UtilityCodeSymbolSearch))
        }
    }

    pub fn code_symbol_search(
        self,
    ) -> Result<either::Either<CodeSymbolImportantRequest, CodeSymbolImportantWideSearch>, ToolError>
    {
        if let ToolInput::RequestImportantSymbols(request_code_symbol_important) = self {
            Ok(either::Either::Left(request_code_symbol_important))
        } else if let ToolInput::RequestImportantSymbolsCodeWide(request_code_symbol_important) =
            self
        {
            Ok(either::Either::Right(request_code_symbol_important))
        } else {
            Err(ToolError::WrongToolInput(ToolType::UtilityCodeSymbolSearch))
        }
    }

    pub fn filter_code_snippets_for_editing(self) -> Result<CodeToEditFilterRequest, ToolError> {
        if let ToolInput::FilterCodeSnippetsForEditing(filter_code_snippets_for_editing) = self {
            Ok(filter_code_snippets_for_editing)
        } else {
            Err(ToolError::WrongToolInput(
                ToolType::FilterCodeSnippetsForEditing,
            ))
        }
    }

    pub fn filter_code_snippets_request(
        self,
    ) -> Result<either::Either<CodeToEditFilterRequest, CodeToEditSymbolRequest>, ToolError> {
        if let ToolInput::FilterCodeSnippetsForEditing(filter_code_snippets_for_editing) = self {
            Ok(either::Left(filter_code_snippets_for_editing))
        } else if let ToolInput::FilterCodeSnippetsForEditingSingleSymbols(
            filter_code_snippets_for_editing,
        ) = self
        {
            Ok(either::Right(filter_code_snippets_for_editing))
        } else {
            Err(ToolError::WrongToolInput(
                ToolType::FilterCodeSnippetsForEditing,
            ))
        }
    }

    pub fn plan_before_code_editing(self) -> Result<PlanningBeforeCodeEditRequest, ToolError> {
        if let ToolInput::PlanningBeforeCodeEdit(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::PlanningBeforeCodeEdit))
        }
    }

    pub fn skill_selector(self) -> Result<SkillSelectorRequest, ToolError> {
        if let ToolInput::SkillInput(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::SkillSelector))
        }
    }
}
