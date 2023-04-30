use std::{
    fmt::Display,
    str::{from_utf8, Utf8Error},
};

use crate::{did_you_mean, Span, Type};
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Error, Diagnostic, Serialize, Deserialize)]
pub enum ParseError {
    /// The parser encountered unexpected tokens, when the code should have
    /// finished. You should remove these or finish adding what you intended
    /// to add.
    #[error("Extra tokens in code.")]
    #[diagnostic(code(nu::parser::extra_tokens), help("Try removing them."))]
    ExtraTokens(#[label = "extra tokens"] Span),

    #[error("Extra positional argument.")]
    #[diagnostic(code(nu::parser::extra_positional), help("Usage: {0}"))]
    ExtraPositional(String, #[label = "extra positional argument"] Span),

    #[error("Required positional parameter after optional parameter")]
    #[diagnostic(code(nu::parser::required_after_optional))]
    RequiredAfterOptional(
        String,
        #[label = "required parameter {0} after optional parameter"] Span,
    ),

    #[error("Unexpected end of code.")]
    #[diagnostic(code(nu::parser::unexpected_eof))]
    UnexpectedEof(String, #[label("expected closing {0}")] Span),

    #[error("Unclosed delimiter.")]
    #[diagnostic(code(nu::parser::unclosed_delimiter))]
    Unclosed(String, #[label("unclosed {0}")] Span),

    #[error("Unbalanced delimiter.")]
    #[diagnostic(code(nu::parser::unbalanced_delimiter))]
    Unbalanced(String, String, #[label("unbalanced {0} and {1}")] Span),

    #[error("Parse mismatch during operation.")]
    #[diagnostic(code(nu::parser::parse_mismatch))]
    Expected(String, #[label("expected {0}")] Span),

    #[error("Type mismatch during operation.")]
    #[diagnostic(code(nu::parser::type_mismatch))]
    Mismatch(String, String, #[label("expected {0}, found {1}")] Span), // expected, found, span

    #[error("The '&&' operator is not supported in Nushell")]
    #[diagnostic(
        code(nu::parser::shell_andand),
        help("use ';' instead of the shell '&&', or 'and' instead of the boolean '&&'")
    )]
    ShellAndAnd(#[label("instead of '&&', use ';' or 'and'")] Span),

    #[error("The '||' operator is not supported in Nushell")]
    #[diagnostic(
        code(nu::parser::shell_oror),
        help("use 'try' instead of the shell '||', or 'or' instead of the boolean '||'")
    )]
    ShellOrOr(#[label("instead of '||', use 'try' or 'or'")] Span),

    #[error("The '2>' shell operation is 'err>' in Nushell.")]
    #[diagnostic(code(nu::parser::shell_err))]
    ShellErrRedirect(#[label("use 'err>' instead of '2>' in Nushell")] Span),

    #[error("The '2>&1' shell operation is 'out+err>' in Nushell.")]
    #[diagnostic(
        code(nu::parser::shell_outerr),
        help("Nushell redirection will write all of stdout before stderr.")
    )]
    ShellOutErrRedirect(#[label("use 'out+err>' instead of '2>&1' in Nushell")] Span),

    #[error("{0} is not supported on values of type {3}")]
    #[diagnostic(code(nu::parser::unsupported_operation))]
    UnsupportedOperationLHS(
        String,
        #[label = "doesn't support this value"] Span,
        #[label("{3}")] Span,
        Type,
    ),

    #[error("{0} is not supported between {3} and {5}.")]
    #[diagnostic(code(nu::parser::unsupported_operation))]
    UnsupportedOperationRHS(
        String,
        #[label = "doesn't support these values"] Span,
        #[label("{3}")] Span,
        Type,
        #[label("{5}")] Span,
        Type,
    ),

    #[error("Capture of mutable variable.")]
    #[diagnostic(code(nu::parser::expected_keyword))]
    CaptureOfMutableVar(#[label("capture of mutable variable")] Span),

    #[error("Expected keyword.")]
    #[diagnostic(code(nu::parser::expected_keyword))]
    ExpectedKeyword(String, #[label("expected {0}")] Span),

    #[error("Unexpected keyword.")]
    #[diagnostic(
        code(nu::parser::unexpected_keyword),
        help("'{0}' keyword is allowed only in a module.")
    )]
    UnexpectedKeyword(String, #[label("unexpected {0}")] Span),

    #[error("Can't create alias to parser keyword.")]
    #[diagnostic(
        code(nu::parser::cant_alias_keyword),
        help("Only the following keywords can be aliased: {0}.")
    )]
    CantAliasKeyword(String, #[label("not supported in alias")] Span),

    #[error("Can't create alias to expression.")]
    #[diagnostic(
        code(nu::parser::cant_alias_expression),
        help("Only command calls can be aliased.")
    )]
    CantAliasExpression(String, #[label("aliasing {0} is not supported")] Span),

    #[error("Unknown operator")]
    #[diagnostic(code(nu::parser::unknown_operator), help("{1}"))]
    UnknownOperator(
        &'static str,
        &'static str,
        #[label("Operator '{0}' not supported")] Span,
    ),

    #[error("Statement used in pipeline.")]
    #[diagnostic(
        code(nu::parser::unexpected_keyword),
        help(
            "'{0}' keyword is not allowed in pipeline. Use '{0}' by itself, outside of a pipeline."
        )
    )]
    BuiltinCommandInPipeline(String, #[label("not allowed in pipeline")] Span),

    #[error("{0} statement used in pipeline.")]
    #[diagnostic(
        code(nu::parser::unexpected_keyword),
        help(
            "Assigning '{1}' to '{2}' does not produce a value to be piped. If the pipeline result is meant to be assigned to '{2}', use '{0} {2} = ({1} | ...)'."
        )
    )]
    AssignInPipeline(String, String, String, #[label("'{0}' in pipeline")] Span),

    #[error("`{0}` used as variable name.")]
    #[diagnostic(
        code(nu::parser::name_is_builtin_var),
        help(
            "'{0}' is the name of a builtin Nushell variable and cannot be used as a variable name"
        )
    )]
    NameIsBuiltinVar(String, #[label("already a builtin variable")] Span),

    #[error("Incorrect value")]
    #[diagnostic(code(nu::parser::incorrect_value), help("{2}"))]
    IncorrectValue(String, #[label("unexpected {0}")] Span, String),

    #[error("Multiple rest params.")]
    #[diagnostic(code(nu::parser::multiple_rest_params))]
    MultipleRestParams(#[label = "multiple rest params"] Span),

    #[error("Variable not found.")]
    #[diagnostic(code(nu::parser::variable_not_found))]
    VariableNotFound(DidYouMean, #[label = "variable not found. {0}"] Span),

    #[error("Variable name not supported.")]
    #[diagnostic(code(nu::parser::variable_not_valid))]
    VariableNotValid(#[label = "variable name can't contain spaces or quotes"] Span),

    #[error("Alias name not supported.")]
    #[diagnostic(code(nu::parser::variable_not_valid))]
    AliasNotValid(
        #[label = "alias name can't be a number, a filesize, or contain a hash # or caret ^"] Span,
    ),

    #[error("Command name not supported.")]
    #[diagnostic(code(nu::parser::variable_not_valid))]
    CommandDefNotValid(
        #[label = "command name can't be a number, a filesize, or contain a hash # or caret ^"]
        Span,
    ),

    #[error("Module not found.")]
    #[diagnostic(
        code(nu::parser::module_not_found),
        help("module files and their paths must be available before your script is run as parsing occurs before anything is evaluated")
    )]
    ModuleNotFound(#[label = "module not found"] Span),

    #[error("Cyclical module import.")]
    #[diagnostic(code(nu::parser::cyclical_module_import), help("{0}"))]
    CyclicalModuleImport(String, #[label = "detected cyclical module import"] Span),

    #[error("Can't export {0} named same as the module.")]
    #[diagnostic(
        code(nu::parser::named_as_module),
        help("Module {1} can't export {0} named the same as the module. Either change the module name, or export `main` custom command.")
    )]
    NamedAsModule(
        String,
        String,
        #[label = "can't export from module {1}"] Span,
    ),

    #[error("Can't export alias defined as 'main'.")]
    #[diagnostic(
        code(nu::parser::export_main_alias_not_allowed),
        help("Exporting aliases as 'main' is not allowed. Either rename the alias or convert it to a custom command.")
    )]
    ExportMainAliasNotAllowed(#[label = "can't export from module"] Span),

    #[error("Active overlay not found.")]
    #[diagnostic(code(nu::parser::active_overlay_not_found))]
    ActiveOverlayNotFound(#[label = "not an active overlay"] Span),

    #[error("Overlay prefix mismatch.")]
    #[diagnostic(
        code(nu::parser::overlay_prefix_mismatch),
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
        help("Requires either an existing overlay, a module, or an import pattern defining a module.")
    )]
    ModuleOrOverlayNotFound(#[label = "not a module or an overlay"] Span),

    #[error("Cannot remove the last overlay.")]
    #[diagnostic(
        code(nu::parser::cant_remove_last_overlay),
        help("At least one overlay must always be active.")
    )]
    CantRemoveLastOverlay(#[label = "this is the last overlay, can't remove it"] Span),

    #[error("Cannot hide default overlay.")]
    #[diagnostic(
        code(nu::parser::cant_hide_default_overlay),
        help("'{0}' is a default overlay. Default overlays cannot be hidden.")
    )]
    CantHideDefaultOverlay(String, #[label = "can't hide overlay"] Span),

    #[error("Cannot add overlay.")]
    #[diagnostic(code(nu::parser::cant_add_overlay_help), help("{0}"))]
    CantAddOverlayHelp(String, #[label = "cannot add this overlay"] Span),

    #[error("Not found.")]
    #[diagnostic(code(nu::parser::not_found))]
    NotFound(#[label = "did not find anything under this name"] Span),

    #[error("Duplicate command definition within a block.")]
    #[diagnostic(code(nu::parser::duplicate_command_def))]
    DuplicateCommandDef(#[label = "defined more than once"] Span),

    #[error("Unknown command.")]
    #[diagnostic(
        code(nu::parser::unknown_command),
        // TODO: actual suggestions like "Did you mean `foo`?"
    )]
    UnknownCommand(#[label = "unknown command"] Span),

    #[error("Non-UTF8 string.")]
    #[diagnostic(code(nu::parser::non_utf8))]
    NonUtf8(#[label = "non-UTF8 string"] Span),

    #[error("The `{0}` command doesn't have flag `{1}`.")]
    #[diagnostic(code(nu::parser::unknown_flag), help("{3}"))]
    UnknownFlag(String, String, #[label = "unknown flag"] Span, String),

    #[error("Unknown type.")]
    #[diagnostic(code(nu::parser::unknown_type))]
    UnknownType(#[label = "unknown type"] Span),

    #[error("Missing flag argument.")]
    #[diagnostic(code(nu::parser::missing_flag_param))]
    MissingFlagParam(String, #[label = "flag missing {0} argument"] Span),

    #[error("Only the last flag in a short flag batch can take an argument.")]
    #[diagnostic(code(nu::parser::only_last_flag_in_batch_can_take_arg))]
    OnlyLastFlagInBatchCanTakeArg(#[label = "only the last flag can take args"] Span),

    #[error("Missing required positional argument.")]
    #[diagnostic(code(nu::parser::missing_positional), help("Usage: {2}"))]
    MissingPositional(String, #[label("missing {0}")] Span, String),

    #[error("Missing argument to `{1}`.")]
    #[diagnostic(code(nu::parser::keyword_missing_arg))]
    KeywordMissingArgument(
        String,
        String,
        #[label("missing {0} value that follows {1}")] Span,
    ),

    #[error("Missing type.")]
    #[diagnostic(code(nu::parser::missing_type))]
    MissingType(#[label = "expected type"] Span),

    #[error("Type mismatch.")]
    #[diagnostic(code(nu::parser::type_mismatch))]
    TypeMismatch(Type, Type, #[label("expected {0}, found {1}")] Span), // expected, found, span

    #[error("Missing required flag.")]
    #[diagnostic(code(nu::parser::missing_required_flag))]
    MissingRequiredFlag(String, #[label("missing required flag {0}")] Span),

    #[error("Incomplete math expression.")]
    #[diagnostic(code(nu::parser::incomplete_math_expression))]
    IncompleteMathExpression(#[label = "incomplete math expression"] Span),

    #[error("Unknown state.")]
    #[diagnostic(code(nu::parser::unknown_state))]
    UnknownState(String, #[label("{0}")] Span),

    #[error("Internal error.")]
    #[diagnostic(code(nu::parser::unknown_state))]
    InternalError(String, #[label("{0}")] Span),

    #[error("Parser incomplete.")]
    #[diagnostic(code(nu::parser::parser_incomplete))]
    IncompleteParser(#[label = "parser support missing for this expression"] Span),

    #[error("Rest parameter needs a name.")]
    #[diagnostic(code(nu::parser::rest_needs_name))]
    RestNeedsName(#[label = "needs a parameter name"] Span),

    #[error("Parameter not correct type.")]
    #[diagnostic(code(nu::parser::parameter_mismatch_type))]
    ParameterMismatchType(
        String,
        String,
        String,
        #[label = "parameter {0} needs to be '{1}' instead of '{2}'"] Span,
    ),

    #[error("Default values should be constant expressions.")]
    #[diagnostic(code(nu::parser::non_constant_default_value))]
    NonConstantDefaultValue(#[label = "expected a constant value"] Span),

    #[error("Extra columns.")]
    #[diagnostic(code(nu::parser::extra_columns))]
    ExtraColumns(
        usize,
        #[label("expected {0} column{}", if *.0 == 1 { "" } else { "s" })] Span,
    ),

    #[error("Missing columns.")]
    #[diagnostic(code(nu::parser::missing_columns))]
    MissingColumns(
        usize,
        #[label("expected {0} column{}", if *.0 == 1 { "" } else { "s" })] Span,
    ),

    #[error("{0}")]
    #[diagnostic(code(nu::parser::assignment_mismatch))]
    AssignmentMismatch(String, String, #[label("{1}")] Span),

    #[error("Missing import pattern.")]
    #[diagnostic(code(nu::parser::missing_import_pattern))]
    MissingImportPattern(#[label = "needs an import pattern"] Span),

    #[error("Wrong import pattern structure.")]
    #[diagnostic(code(nu::parser::missing_import_pattern))]
    WrongImportPattern(#[label = "invalid import pattern structure"] Span),

    #[error("Export not found.")]
    #[diagnostic(code(nu::parser::export_not_found))]
    ExportNotFound(#[label = "could not find imports"] Span),

    #[error("File not found")]
    #[diagnostic(
        code(nu::parser::sourced_file_not_found),
        help("sourced files need to be available before your script is run")
    )]
    SourcedFileNotFound(String, #[label("File not found: {0}")] Span),

    #[error("File not found")]
    #[diagnostic(
        code(nu::parser::registered_file_not_found),
        help("registered files need to be available before your script is run")
    )]
    RegisteredFileNotFound(String, #[label("File not found: {0}")] Span),

    #[error("File not found")]
    #[diagnostic(code(nu::parser::file_not_found))]
    FileNotFound(String, #[label("File not found: {0}")] Span),

    /// Error while trying to read a file
    ///
    /// ## Resolution
    ///
    /// The error will show the result from a file operation
    #[error("Error trying to read file")]
    #[diagnostic(code(nu::shell::error_reading_file))]
    ReadingFile(String, #[label("{0}")] Span),

    /// Tried assigning non-constant value to a constant
    ///
    /// ## Resolution
    ///
    /// Only a subset of expressions are allowed to be assigned as a constant during parsing.
    #[error("Not a constant.")]
    #[diagnostic(
        code(nu::parser::not_a_constant),
        help("Only a subset of expressions are allowed constants during parsing. Try using the 'const' command or typing the value literally.")
    )]
    NotAConstant(#[label = "Value is not a parse-time constant"] Span),

    #[error("Invalid literal")] // <problem> in <entity>.
    #[diagnostic()]
    InvalidLiteral(String, String, #[label("{0} in {1}")] Span),

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
            ParseError::Unbalanced(_, _, s) => *s,
            ParseError::Expected(_, s) => *s,
            ParseError::Mismatch(_, _, s) => *s,
            ParseError::UnsupportedOperationLHS(_, _, s, _) => *s,
            ParseError::UnsupportedOperationRHS(_, _, _, _, s, _) => *s,
            ParseError::ExpectedKeyword(_, s) => *s,
            ParseError::UnexpectedKeyword(_, s) => *s,
            ParseError::CantAliasKeyword(_, s) => *s,
            ParseError::CantAliasExpression(_, s) => *s,
            ParseError::BuiltinCommandInPipeline(_, s) => *s,
            ParseError::AssignInPipeline(_, _, _, s) => *s,
            ParseError::NameIsBuiltinVar(_, s) => *s,
            ParseError::CaptureOfMutableVar(s) => *s,
            ParseError::IncorrectValue(_, s, _) => *s,
            ParseError::MultipleRestParams(s) => *s,
            ParseError::VariableNotFound(_, s) => *s,
            ParseError::VariableNotValid(s) => *s,
            ParseError::AliasNotValid(s) => *s,
            ParseError::CommandDefNotValid(s) => *s,
            ParseError::ModuleNotFound(s) => *s,
            ParseError::NamedAsModule(_, _, s) => *s,
            ParseError::ExportMainAliasNotAllowed(s) => *s,
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
            ParseError::UnknownFlag(_, _, s, _) => *s,
            ParseError::RequiredAfterOptional(_, s) => *s,
            ParseError::UnknownType(s) => *s,
            ParseError::MissingFlagParam(_, s) => *s,
            ParseError::OnlyLastFlagInBatchCanTakeArg(s) => *s,
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
            ParseError::NonConstantDefaultValue(s) => *s,
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
            ParseError::ShellAndAnd(s) => *s,
            ParseError::ShellOrOr(s) => *s,
            ParseError::ShellErrRedirect(s) => *s,
            ParseError::ShellOutErrRedirect(s) => *s,
            ParseError::UnknownOperator(_, _, s) => *s,
            ParseError::InvalidLiteral(_, _, s) => *s,
            ParseError::NotAConstant(s) => *s,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DidYouMean(Option<String>);

fn did_you_mean_impl(possibilities_bytes: &[&[u8]], input_bytes: &[u8]) -> Option<String> {
    let input = from_utf8(input_bytes).ok()?;
    let possibilities = possibilities_bytes
        .iter()
        .map(|p| from_utf8(p))
        .collect::<Result<Vec<&str>, Utf8Error>>()
        .ok()?;
    did_you_mean(&possibilities, input)
}
impl DidYouMean {
    pub fn new(possibilities_bytes: &[&[u8]], input_bytes: &[u8]) -> DidYouMean {
        DidYouMean(did_you_mean_impl(possibilities_bytes, input_bytes))
    }
}

impl From<Option<String>> for DidYouMean {
    fn from(value: Option<String>) -> Self {
        Self(value)
    }
}

impl Display for DidYouMean {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(suggestion) = &self.0 {
            write!(f, "Did you mean '{}'?", suggestion)
        } else {
            write!(f, "")
        }
    }
}
