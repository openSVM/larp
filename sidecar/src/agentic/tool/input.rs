use super::{
    base::ToolType,
    code_edit::{find::FindCodeSelectionInput, types::CodeEdit},
    code_symbol::{
        correctness::CodeCorrectnessRequest,
        error_fix::CodeEditingErrorRequest,
        important::{
            CodeSymbolImportantRequest, CodeSymbolImportantWideSearch, CodeSymbolUtilityRequest,
        },
    },
    editor::apply::EditorApplyRequest,
    errors::ToolError,
    filtering::broker::{CodeToEditFilterRequest, CodeToEditSymbolRequest},
    grep::file::FindInFileRequest,
    lsp::{
        diagnostics::LSPDiagnosticsInput,
        gotodefintion::GoToDefinitionRequest,
        gotoimplementations::GoToImplementationRequest,
        gotoreferences::{GoToReferencesRequest, GoToReferencesResponse},
        open_file::OpenFileRequest,
        quick_fix::{GetQuickFixRequest, LSPQuickFixInvocationRequest},
    },
    rerank::base::ReRankEntriesForBroker,
};

#[derive(Debug, Clone)]
pub enum ToolInput {
    CodeEditing(CodeEdit),
    LSPDiagnostics(LSPDiagnosticsInput),
    FindCodeSnippets(FindCodeSelectionInput),
    ReRank(ReRankEntriesForBroker),
    CodeSymbolUtilitySearch(CodeSymbolUtilityRequest),
    RequestImportantSymbols(CodeSymbolImportantRequest),
    RequestImportantSybmolsCodeWide(CodeSymbolImportantWideSearch),
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
}

impl ToolInput {
    pub fn tool_type(&self) -> ToolType {
        match self {
            ToolInput::CodeEditing(_) => ToolType::CodeEditing,
            ToolInput::LSPDiagnostics(_) => ToolType::LSPDiagnostics,
            ToolInput::FindCodeSnippets(_) => ToolType::FindCodeSnippets,
            ToolInput::ReRank(_) => ToolType::ReRank,
            ToolInput::RequestImportantSymbols(_) => ToolType::RequestImportantSymbols,
            ToolInput::RequestImportantSybmolsCodeWide(_) => ToolType::FindCodeSymbolsCodeBaseWide,
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
        }
    }

    pub fn code_editing_error(self) -> Result<CodeEditingErrorRequest, ToolError> {
        if let ToolInput::CodeEditingError(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn code_correctness_action(self) -> Result<CodeCorrectnessRequest, ToolError> {
        if let ToolInput::CodeCorrectnessAction(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn quick_fix_invocation_request(self) -> Result<LSPQuickFixInvocationRequest, ToolError> {
        if let ToolInput::QuickFixInvocationRequest(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn quick_fix_request(self) -> Result<GetQuickFixRequest, ToolError> {
        if let ToolInput::QuickFixRequest(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn editor_apply_changes(self) -> Result<EditorApplyRequest, ToolError> {
        if let ToolInput::EditorApplyChange(editor_apply_request) = self {
            Ok(editor_apply_request)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn symbol_implementations(self) -> Result<GoToImplementationRequest, ToolError> {
        if let ToolInput::SymbolImplementations(symbol_implementations) = self {
            Ok(symbol_implementations)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn reference_request(self) -> Result<GoToReferencesRequest, ToolError> {
        if let ToolInput::GoToReference(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn grep_single_file(self) -> Result<FindInFileRequest, ToolError> {
        if let ToolInput::GrepSingleFile(grep_single_file) = self {
            Ok(grep_single_file)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn is_file_open(self) -> Result<OpenFileRequest, ToolError> {
        if let ToolInput::OpenFile(open_file) = self {
            Ok(open_file)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn is_go_to_definition(self) -> Result<GoToDefinitionRequest, ToolError> {
        if let ToolInput::GoToDefinition(definition_request) = self {
            Ok(definition_request)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn is_code_edit(self) -> Result<CodeEdit, ToolError> {
        if let ToolInput::CodeEditing(code_edit) = self {
            Ok(code_edit)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn is_lsp_diagnostics(self) -> Result<LSPDiagnosticsInput, ToolError> {
        if let ToolInput::LSPDiagnostics(lsp_diagnostics) = self {
            Ok(lsp_diagnostics)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn is_code_find(self) -> Result<FindCodeSelectionInput, ToolError> {
        if let ToolInput::FindCodeSnippets(find_code_snippets) = self {
            Ok(find_code_snippets)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn is_rerank(self) -> Result<ReRankEntriesForBroker, ToolError> {
        if let ToolInput::ReRank(rerank) = self {
            Ok(rerank)
        } else {
            Err(ToolError::WrongToolInput)
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
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn code_symbol_search(
        self,
    ) -> Result<either::Either<CodeSymbolImportantRequest, CodeSymbolImportantWideSearch>, ToolError>
    {
        if let ToolInput::RequestImportantSymbols(request_code_symbol_important) = self {
            Ok(either::Either::Left(request_code_symbol_important))
        } else if let ToolInput::RequestImportantSybmolsCodeWide(request_code_symbol_important) =
            self
        {
            Ok(either::Either::Right(request_code_symbol_important))
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn code_symbol_important(self) -> Result<CodeSymbolImportantRequest, ToolError> {
        if let ToolInput::RequestImportantSymbols(request_code_symbol_important) = self {
            Ok(request_code_symbol_important)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn codebase_wide_important_symbols(
        self,
    ) -> Result<CodeSymbolImportantWideSearch, ToolError> {
        if let ToolInput::RequestImportantSybmolsCodeWide(request_code_symbol_important) = self {
            Ok(request_code_symbol_important)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn filter_code_snippets_for_editing(self) -> Result<CodeToEditFilterRequest, ToolError> {
        if let ToolInput::FilterCodeSnippetsForEditing(filter_code_snippets_for_editing) = self {
            Ok(filter_code_snippets_for_editing)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn filter_code_snippets_single_symbol(self) -> Result<CodeToEditSymbolRequest, ToolError> {
        if let ToolInput::FilterCodeSnippetsForEditingSingleSymbols(
            filter_code_snippets_for_editing,
        ) = self
        {
            Ok(filter_code_snippets_for_editing)
        } else {
            Err(ToolError::WrongToolInput)
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
            Err(ToolError::WrongToolInput)
        }
    }
}
