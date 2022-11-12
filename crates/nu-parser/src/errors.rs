use miette::Diagnostic;
use nu_protocol::{Span, Type};
use thiserror::Error;

#[derive(Clone, Debug, Error, Diagnostic)]
pub enum ParseError {
    /// The parser encountered unexpected tokens, when the code should have
    /// finished. You should remove these or finish adding what you intended
    /// to add.
    #[error("Extra tokens in code.")]
    #[diagnostic(
        code(nu::parser::extra_tokens),
        url(docsrs),
        help("Try removing them.")
    )]
    ExtraTokens(#[label = "extra tokens"] Span),

    #[error("Extra positional argument.")]
    #[diagnostic(code(nu::parser::extra_positional), url(docsrs), help("Usage: {0}"))]
    ExtraPositional(String, #[label = "extra positional argument"] Span),

    #[error("Require positional parameter after optional parameter")]
    #[diagnostic(code(nu::parser::required_after_optional), url(docsrs))]
    RequiredAfterOptional(
        String,
        #[label = "required parameter {0} after optional parameter"] Span,
    ),

    #[error("Unexpected end of code.")]
    #[diagnostic(code(nu::parser::unexpected_eof), url(docsrs))]
    UnexpectedEof(String, #[label("expected closing {0}")] Span),

    #[error("Unclosed delimiter.")]
    #[diagnostic(code(nu::parser::unclosed_delimiter), url(docsrs))]
    Unclosed(String, #[label("unclosed {0}")] Span),

    #[error("Parse mismatch during operation.")]
    #[diagnostic(code(nu::parser::parse_mismatch), url(docsrs))]
    Expected(String, #[label("expected {0}")] Span),

    #[error("Type mismatch during operation.")]
    #[diagnostic(code(nu::parser::type_mismatch), url(docsrs))]
    Mismatch(String, String, #[label("expected {0}, found {1}")] Span), // expected, found, span

    #[error("Types mismatched for operation.")]
    #[diagnostic(
        code(nu::parser::unsupported_operation),
        url(docsrs),
        help("Change {2} or {4} to be the right types and try again.")
    )]
    UnsupportedOperation(
        #[label = "doesn't support these values."] Span,
        #[label("{2}")] Span,
        Type,
        #[label("{4}")] Span,
        Type,
    ),

    #[error("Capture of mutable variable.")]
    #[diagnostic(code(nu::parser::expected_keyword), url(docsrs))]
    CaptureOfMutableVar(#[label("capture of mutable variable")] Span),

    #[error("Expected keyword.")]
    #[diagnostic(code(nu::parser::expected_keyword), url(docsrs))]
    ExpectedKeyword(String, #[label("expected {0}")] Span),

    #[error("Unexpected keyword.")]
    #[diagnostic(
        code(nu::parser::unexpected_keyword),
        url(docsrs),
        help("'{0}' keyword is allowed only in a module.")
    )]
    UnexpectedKeyword(String, #[label("unexpected {0}")] Span),

    #[error("Statement used in pipeline.")]
    #[diagnostic(
        code(nu::parser::unexpected_keyword),
        url(docsrs),
        help(
            "'{0}' keyword is not allowed in pipeline. Use '{0}' by itself, outside of a pipeline."
        )
    )]
    BuiltinCommandInPipeline(String, #[label("not allowed in pipeline")] Span),

    #[error("Let statement used in pipeline.")]
    #[diagnostic(
        code(nu::parser::unexpected_keyword),
        url(docsrs),
        help(
            "Assigning '{0}' to '{1}' does not produce a value to be piped. If the pipeline result is meant to be assigned to '{1}', use 'let {1} = ({0} | ...)'."
        )
    )]
    LetInPipeline(String, String, #[label("let in pipeline")] Span),

    #[error("Mut statement used in pipeline.")]
    #[diagnostic(
        code(nu::parser::unexpected_keyword),
        url(docsrs),
        help(
            "Assigning '{0}' to '{1}' does not produce a value to be piped. If the pipeline result is meant to be assigned to '{1}', use 'mut {1} = ({0} | ...)'."
        )
    )]
    MutInPipeline(String, String, #[label("let in pipeline")] Span),

    #[error("Let used with builtin variable name.")]
    #[diagnostic(
        code(nu::parser::let_builtin_var),
        url(docsrs),
        help("'{0}' is the name of a builtin Nushell variable. `let` cannot assign to it.")
    )]
    LetBuiltinVar(String, #[label("already a builtin variable")] Span),

    #[error("Mut used with builtin variable name.")]
    #[diagnostic(
        code(nu::parser::let_builtin_var),
        url(docsrs),
        help("'{0}' is the name of a builtin Nushell variable. `mut` cannot assign to it.")
    )]
    MutBuiltinVar(String, #[label("already a builtin variable")] Span),

    #[error("Incorrect value")]
    #[diagnostic(code(nu::parser::incorrect_value), url(docsrs), help("{2}"))]
    IncorrectValue(String, #[label("unexpected {0}")] Span, String),

    #[error("Multiple rest params.")]
    #[diagnostic(code(nu::parser::multiple_rest_params), url(docsrs))]
    MultipleRestParams(#[label = "multiple rest params"] Span),

    #[error("Variable not found.")]
    #[diagnostic(code(nu::parser::variable_not_found), url(docsrs))]
    VariableNotFound(#[label = "variable not found"] Span),

    #[error("Variable name not supported.")]
    #[diagnostic(code(nu::parser::variable_not_valid), url(docsrs))]
    VariableNotValid(#[label = "variable name can't contain spaces or quotes"] Span),

    #[error("Alias name not supported.")]
    #[diagnostic(code(nu::parser::variable_not_valid), url(docsrs))]
    AliasNotValid(#[label = "alias name can't be a number or a filesize"] Span),

    #[error("Module not found.")]
    #[diagnostic(
        code(nu::parser::module_not_found),
        url(docsrs),
        help("module files and their paths must be available before your script is run as parsing occurs before anything is evaluated")
    )]
    ModuleNotFound(#[label = "module not found"] Span),

    #[error("Cyclical module import.")]
    #[diagnostic(code(nu::parser::cyclical_module_import), url(docsrs), help("{0}"))]
    CyclicalModuleImport(String, #[label = "detected cyclical module import"] Span),

    #[error("Active overlay not found.")]
    #[diagnostic(code(nu::parser::active_overlay_not_found), url(docsrs))]
    ActiveOverlayNotFound(#[label = "not an active overlay"] Span),

    #[error("Overlay prefix mismatch.")]
    #[diagnostic(
        code(nu::parser::overlay_prefix_mismatch),
        url(docsrs),
        help("Overlay {0} already exists {1} a prefix. To add it again, do it {1} the --prefix flag.")
    )]
    OverlayPrefixMismatch(
        String,
        String,
        #[label = "already exists {1} a prefix"] Span,
    ),

    #[error("Module or overlay not found.")]
    #[diagnostic(
        code(nu::parser::module_or_overlay_not_found),
        url(docsrs),
        help("Requires either an existing overlay, a module, or an import pattern defining a module.")
    )]
    ModuleOrOverlayNotFound(#[label = "not a module or an overlay"] Span),

    #[error("Cannot remove the last overlay.")]
    #[diagnostic(
        code(nu::parser::cant_remove_last_overlay),
        url(docsrs),
        help("At least one overlay must always be active.")
    )]
    CantRemoveLastOverlay(#[label = "this is the last overlay, can't remove it"] Span),

    #[error("Cannot hide default overlay.")]
    #[diagnostic(
        code(nu::parser::cant_hide_default_overlay),
        url(docsrs),
        help("'{0}' is a default overlay. Default overlays cannot be hidden.")
    )]
    CantHideDefaultOverlay(String, #[label = "can't hide overlay"] Span),

    #[error("Cannot add overlay.")]
    #[diagnostic(code(nu::parser::cant_add_overlay_help), url(docsrs), help("{0}"))]
    CantAddOverlayHelp(String, #[label = "cannot add this overlay"] Span),

    #[error("Not found.")]
    #[diagnostic(code(nu::parser::not_found), url(docsrs))]
    NotFound(#[label = "did not find anything under this name"] Span),

    #[error("Duplicate command definition within a block.")]
    #[diagnostic(code(nu::parser::duplicate_command_def), url(docsrs))]
    DuplicateCommandDef(#[label = "defined more than once"] Span),

    #[error("Unknown command.")]
    #[diagnostic(
        code(nu::parser::unknown_command),
        url(docsrs),
        // TODO: actual suggestions like "Did you mean `foo`?"
    )]
    UnknownCommand(#[label = "unknown command"] Span),

    #[error("Non-UTF8 string.")]
    #[diagnostic(code(nu::parser::non_utf8), url(docsrs))]
    NonUtf8(#[label = "non-UTF8 string"] Span),

    #[error("The `{0}` command doesn't have flag `{1}`.")]
    #[diagnostic(
        code(nu::parser::unknown_flag),
        url(docsrs),
        help("use {0} --help for a list of flags")
    )]
    UnknownFlag(String, String, #[label = "unknown flag"] Span),

    #[error("Unknown type.")]
    #[diagnostic(code(nu::parser::unknown_type), url(docsrs))]
    UnknownType(#[label = "unknown type"] Span),

    #[error("Missing flag argument.")]
    #[diagnostic(code(nu::parser::missing_flag_param), url(docsrs))]
    MissingFlagParam(String, #[label = "flag missing {0} argument"] Span),

    #[error("Batches of short flags can't take arguments.")]
    #[diagnostic(code(nu::parser::short_flag_arg_cant_take_arg), url(docsrs))]
    ShortFlagBatchCantTakeArg(#[label = "short flag batches can't take args"] Span),

    #[error("Missing required positional argument.")]
    #[diagnostic(code(nu::parser::missing_positional), url(docsrs), help("Usage: {2}"))]
    MissingPositional(String, #[label("missing {0}")] Span, String),

    #[error("Missing argument to `{1}`.")]
    #[diagnostic(code(nu::parser::keyword_missing_arg), url(docsrs))]
    KeywordMissingArgument(
        String,
        String,
        #[label("missing {0} value that follows {1}")] Span,
    ),

    #[error("Missing type.")]
    #[diagnostic(code(nu::parser::missing_type), url(docsrs))]
    MissingType(#[label = "expected type"] Span),

    #[error("Type mismatch.")]
    #[diagnostic(code(nu::parser::type_mismatch), url(docsrs))]
    TypeMismatch(Type, Type, #[label("expected {0:?}, found {1:?}")] Span), // expected, found, span

    #[error("Missing required flag.")]
    #[diagnostic(code(nu::parser::missing_required_flag), url(docsrs))]
    MissingRequiredFlag(String, #[label("missing required flag {0}")] Span),

    #[error("Incomplete math expression.")]
    #[diagnostic(code(nu::parser::incomplete_math_expression), url(docsrs))]
    IncompleteMathExpression(#[label = "incomplete math expression"] Span),

    #[error("Unknown state.")]
    #[diagnostic(code(nu::parser::unknown_state), url(docsrs))]
    UnknownState(String, #[label("{0}")] Span),

    #[error("Internal error.")]
    #[diagnostic(code(nu::parser::unknown_state), url(docsrs))]
    InternalError(String, #[label("{0}")] Span),

    #[error("Parser incomplete.")]
    #[diagnostic(code(nu::parser::parser_incomplete), url(docsrs))]
    IncompleteParser(#[label = "parser support missing for this expression"] Span),

    #[error("Rest parameter needs a name.")]
    #[diagnostic(code(nu::parser::rest_needs_name), url(docsrs))]
    RestNeedsName(#[label = "needs a parameter name"] Span),

    #[error("Parameter not correct type.")]
    #[diagnostic(code(nu::parser::parameter_mismatch_type), url(docsrs))]
    ParameterMismatchType(
        String,
        String,
        String,
        #[label = "parameter {0} needs to be '{1}' instead of '{2}'"] Span,
    ),

    #[error("Extra columns.")]
    #[diagnostic(code(nu::parser::extra_columns), url(docsrs))]
    ExtraColumns(
        usize,
        #[label("expected {0} column{}", if *.0 == 1 { "" } else { "s" })] Span,
    ),

    #[error("Missing columns.")]
    #[diagnostic(code(nu::parser::missing_columns), url(docsrs))]
    MissingColumns(
        usize,
        #[label("expected {0} column{}", if *.0 == 1 { "" } else { "s" })] Span,
    ),

    #[error("{0}")]
    #[diagnostic(code(nu::parser::assignment_mismatch), url(docsrs))]
    AssignmentMismatch(String, String, #[label("{1}")] Span),

    #[error("Missing import pattern.")]
    #[diagnostic(code(nu::parser::missing_import_pattern), url(docsrs))]
    MissingImportPattern(#[label = "needs an import pattern"] Span),

    #[error("Wrong import pattern structure.")]
    #[diagnostic(code(nu::parser::missing_import_pattern), url(docsrs))]
    WrongImportPattern(#[label = "invalid import pattern structure"] Span),

    #[error("Export not found.")]
    #[diagnostic(code(nu::parser::export_not_found), url(docsrs))]
    ExportNotFound(#[label = "could not find imports"] Span),

    #[error("File not found")]
    #[diagnostic(
        code(nu::parser::sourced_file_not_found),
        url(docsrs),
        help("sourced files need to be available before your script is run")
    )]
    SourcedFileNotFound(String, #[label("File not found: {0}")] Span),

    #[error("File not found")]
    #[diagnostic(
        code(nu::parser::registered_file_not_found),
        url(docsrs),
        help("registered files need to be available before your script is run")
    )]
    RegisteredFileNotFound(String, #[label("File not found: {0}")] Span),

    #[error("File not found")]
    #[diagnostic(code(nu::parser::file_not_found), url(docsrs))]
    FileNotFound(String, #[label("File not found: {0}")] Span),

    /// Error while trying to read a file
    ///
    /// ## Resolution
    ///
    /// The error will show the result from a file operation
    #[error("Error trying to read file")]
    #[diagnostic(code(nu::shell::error_reading_file), url(docsrs))]
    ReadingFile(String, #[label("{0}")] Span),

    #[error("{0}")]
    #[diagnostic()]
    LabeledError(String, String, #[label("{1}")] Span),
}

impl ParseError {
    pub fn span(&self) -> Span {
        match self {
            ParseError::ExtraTokens(s) => *s,
            ParseError::ExtraPositional(_, s) => *s,
            ParseError::UnexpectedEof(_, s) => *s,
            ParseError::Unclosed(_, s) => *s,
            ParseError::Expected(_, s) => *s,
            ParseError::Mismatch(_, _, s) => *s,
            ParseError::UnsupportedOperation(_, _, _, s, _) => *s,
            ParseError::ExpectedKeyword(_, s) => *s,
            ParseError::UnexpectedKeyword(_, s) => *s,
            ParseError::BuiltinCommandInPipeline(_, s) => *s,
            ParseError::LetInPipeline(_, _, s) => *s,
            ParseError::MutInPipeline(_, _, s) => *s,
            ParseError::LetBuiltinVar(_, s) => *s,
            ParseError::MutBuiltinVar(_, s) => *s,
            ParseError::CaptureOfMutableVar(s) => *s,
            ParseError::IncorrectValue(_, s, _) => *s,
            ParseError::MultipleRestParams(s) => *s,
            ParseError::VariableNotFound(s) => *s,
            ParseError::VariableNotValid(s) => *s,
            ParseError::AliasNotValid(s) => *s,
            ParseError::ModuleNotFound(s) => *s,
            ParseError::CyclicalModuleImport(_, s) => *s,
            ParseError::ModuleOrOverlayNotFound(s) => *s,
            ParseError::ActiveOverlayNotFound(s) => *s,
            ParseError::OverlayPrefixMismatch(_, _, s) => *s,
            ParseError::CantRemoveLastOverlay(s) => *s,
            ParseError::CantHideDefaultOverlay(_, s) => *s,
            ParseError::CantAddOverlayHelp(_, s) => *s,
            ParseError::NotFound(s) => *s,
            ParseError::DuplicateCommandDef(s) => *s,
            ParseError::UnknownCommand(s) => *s,
            ParseError::NonUtf8(s) => *s,
            ParseError::UnknownFlag(_, _, s) => *s,
            ParseError::RequiredAfterOptional(_, s) => *s,
            ParseError::UnknownType(s) => *s,
            ParseError::MissingFlagParam(_, s) => *s,
            ParseError::ShortFlagBatchCantTakeArg(s) => *s,
            ParseError::MissingPositional(_, s, _) => *s,
            ParseError::KeywordMissingArgument(_, _, s) => *s,
            ParseError::MissingType(s) => *s,
            ParseError::TypeMismatch(_, _, s) => *s,
            ParseError::MissingRequiredFlag(_, s) => *s,
            ParseError::IncompleteMathExpression(s) => *s,
            ParseError::UnknownState(_, s) => *s,
            ParseError::InternalError(_, s) => *s,
            ParseError::IncompleteParser(s) => *s,
            ParseError::RestNeedsName(s) => *s,
            ParseError::ParameterMismatchType(_, _, _, s) => *s,
            ParseError::ExtraColumns(_, s) => *s,
            ParseError::MissingColumns(_, s) => *s,
            ParseError::AssignmentMismatch(_, _, s) => *s,
            ParseError::MissingImportPattern(s) => *s,
            ParseError::WrongImportPattern(s) => *s,
            ParseError::ExportNotFound(s) => *s,
            ParseError::SourcedFileNotFound(_, s) => *s,
            ParseError::RegisteredFileNotFound(_, s) => *s,
            ParseError::FileNotFound(_, s) => *s,
            ParseError::ReadingFile(_, s) => *s,
            ParseError::LabeledError(_, _, s) => *s,
        }
    }
}
