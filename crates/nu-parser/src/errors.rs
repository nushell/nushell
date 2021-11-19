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
    #[diagnostic(code(nu::parser::extra_positional), url(docsrs))]
    ExtraPositional(#[label = "extra positional argument"] Span),

    #[error("Unexpected end of code.")]
    #[diagnostic(code(nu::parser::unexpected_eof), url(docsrs))]
    UnexpectedEof(String, #[label("expected closing {0}")] Span),

    #[error("Unclosed delimiter.")]
    #[diagnostic(code(nu::parser::unclosed_delimiter), url(docsrs))]
    Unclosed(String, #[label("unclosed {0}")] Span),

    #[error("Unknown statement.")]
    #[diagnostic(code(nu::parser::unknown_statement), url(docsrs))]
    UnknownStatement(#[label("unknown statement")] Span),

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

    #[error("Expected keyword.")]
    #[diagnostic(code(nu::parser::expected_keyword), url(docsrs))]
    ExpectedKeyword(String, #[label("expected {0}")] Span),

    #[error("Unexpected keyword.")]
    #[diagnostic(
        code(nu::parser::unexpected_keyword),
        url(docsrs),
        help("'export' keyword is allowed only in a module.")
    )]
    UnexpectedKeyword(String, #[label("unexpected {0}")] Span),

    #[error("Multiple rest params.")]
    #[diagnostic(code(nu::parser::multiple_rest_params), url(docsrs))]
    MultipleRestParams(#[label = "multiple rest params"] Span),

    #[error("Variable not found.")]
    #[diagnostic(code(nu::parser::variable_not_found), url(docsrs))]
    VariableNotFound(#[label = "variable not found"] Span),

    #[error("Variable name not supported.")]
    #[diagnostic(code(nu::parser::variable_not_valid), url(docsrs))]
    VariableNotValid(#[label = "variable name can't contain spaces or quotes"] Span),

    #[error("Module not found.")]
    #[diagnostic(code(nu::parser::module_not_found), url(docsrs))]
    ModuleNotFound(#[label = "module not found"] Span),

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
    #[diagnostic(code(nu::parser::unknown_flag), url(docsrs))]
    UnknownFlag(String, String, #[label = "unknown flag"] Span),

    #[error("Unknown type.")]
    #[diagnostic(code(nu::parser::unknown_type), url(docsrs))]
    UnknownType(#[label = "unknown type"] Span),

    #[error("Missing flag param.")]
    #[diagnostic(code(nu::parser::missing_flag_param), url(docsrs))]
    MissingFlagParam(#[label = "flag missing param"] Span),

    #[error("Batches of short flags can't take arguments.")]
    #[diagnostic(code(nu::parser::short_flag_arg_cant_take_arg), url(docsrs))]
    ShortFlagBatchCantTakeArg(#[label = "short flag batches can't take args"] Span),

    #[error("Missing required positional argument.")]
    #[diagnostic(code(nu::parser::missing_positional), url(docsrs))]
    MissingPositional(String, #[label("missing {0}")] Span),

    #[error("Missing argument to `{0}`.")]
    #[diagnostic(code(nu::parser::keyword_missing_arg), url(docsrs))]
    KeywordMissingArgument(String, #[label("missing value that follows {0}")] Span),

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
    #[diagnostic(code(nu::parser::export_not_found), url(docsrs))]
    FileNotFound(String),

    #[error("Plugin error")]
    #[diagnostic(code(nu::parser::plugin_error), url(docsrs))]
    PluginError(String),
}
