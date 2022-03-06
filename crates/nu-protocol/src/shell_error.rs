use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{ast::Operator, Span, Type};

/// The fundamental error type for the evaluation engine. These cases represent different kinds of errors
/// the evaluator might face, along with helpful spans to label. An error renderer will take this error value
/// and pass it into an error viewer to display to the user.
#[derive(Debug, Clone, Error, Diagnostic, Serialize, Deserialize)]
pub enum ShellError {
    #[error("Type mismatch during operation.")]
    #[diagnostic(code(nu::shell::type_mismatch), url(docsrs))]
    OperatorMismatch {
        #[label = "type mismatch for operator"]
        op_span: Span,
        lhs_ty: Type,
        #[label("{lhs_ty}")]
        lhs_span: Span,
        rhs_ty: Type,
        #[label("{rhs_ty}")]
        rhs_span: Span,
    },

    #[error("Operator overflow.")]
    #[diagnostic(code(nu::shell::operator_overflow), url(docsrs))]
    OperatorOverflow(String, #[label = "{0}"] Span),

    #[error("Pipeline mismatch.")]
    #[diagnostic(code(nu::shell::pipeline_mismatch), url(docsrs))]
    PipelineMismatch(
        String,
        #[label("expected: {0}")] Span,
        #[label("value originates from here")] Span,
    ),

    #[error("Type mismatch")]
    #[diagnostic(code(nu::shell::type_mismatch), url(docsrs))]
    TypeMismatch(String, #[label = "needs {0}"] Span),

    #[error("Unsupported operator: {0}.")]
    #[diagnostic(code(nu::shell::unsupported_operator), url(docsrs))]
    UnsupportedOperator(Operator, #[label = "unsupported operator"] Span),

    #[error("Unsupported operator: {0}.")]
    #[diagnostic(code(nu::shell::unknown_operator), url(docsrs))]
    UnknownOperator(String, #[label = "unsupported operator"] Span),

    #[error("Missing parameter: {0}.")]
    #[diagnostic(code(nu::shell::missing_parameter), url(docsrs))]
    MissingParameter(String, #[label = "missing parameter: {0}"] Span),

    // Be cautious, as flags can share the same span, resulting in a panic (ex: `rm -pt`)
    #[error("Incompatible parameters.")]
    #[diagnostic(code(nu::shell::incompatible_parameters), url(docsrs))]
    IncompatibleParameters {
        left_message: String,
        #[label("{left_message}")]
        left_span: Span,
        right_message: String,
        #[label("{right_message}")]
        right_span: Span,
    },

    #[error("Delimiter error")]
    #[diagnostic(code(nu::shell::delimiter_error), url(docsrs))]
    DelimiterError(String, #[label("{0}")] Span),

    #[error("Incompatible parameters.")]
    #[diagnostic(code(nu::shell::incompatible_parameters), url(docsrs))]
    IncompatibleParametersSingle(String, #[label = "{0}"] Span),

    #[error("Feature not enabled.")]
    #[diagnostic(code(nu::shell::feature_not_enabled), url(docsrs))]
    FeatureNotEnabled(#[label = "feature not enabled"] Span),

    #[error("Running external commands not supported")]
    #[diagnostic(code(nu::shell::external_commands), url(docsrs))]
    ExternalNotSupported(#[label = "external not supported"] Span),

    #[error("Invalid Probability.")]
    #[diagnostic(code(nu::shell::invalid_probability), url(docsrs))]
    InvalidProbability(#[label = "invalid probability"] Span),

    #[error("Invalid range {0}..{1}")]
    #[diagnostic(code(nu::shell::invalid_range), url(docsrs))]
    InvalidRange(String, String, #[label = "expected a valid range"] Span),

    // Only use this one if we Nushell completely falls over and hits a state that isn't possible or isn't recoverable
    #[error("Nushell failed: {0}.")]
    #[diagnostic(code(nu::shell::nushell_failed), url(docsrs))]
    NushellFailed(String),

    // Only use this one if we Nushell completely falls over and hits a state that isn't possible or isn't recoverable
    #[error("Nushell failed: {0}.")]
    #[diagnostic(code(nu::shell::nushell_failed), url(docsrs))]
    NushellFailedSpanned(String, String, #[label = "{1}"] Span),

    #[error("Variable not found")]
    #[diagnostic(code(nu::shell::variable_not_found), url(docsrs))]
    VariableNotFoundAtRuntime(#[label = "variable not found"] Span),

    #[error("Environment variable '{0}' not found")]
    #[diagnostic(code(nu::shell::env_variable_not_found), url(docsrs))]
    EnvVarNotFoundAtRuntime(String, #[label = "environment variable not found"] Span),

    #[error("Not found.")]
    #[diagnostic(code(nu::parser::not_found), url(docsrs))]
    NotFound(#[label = "did not find anything under this name"] Span),

    #[error("Can't convert to {0}.")]
    #[diagnostic(code(nu::shell::cant_convert), url(docsrs))]
    CantConvert(String, String, #[label("can't convert {1} to {0}")] Span),

    #[error("{0} is not representable as a string.")]
    #[diagnostic(
        code(nu::shell::env_var_not_a_string),
        url(docsrs),
        help(
            r#"The '{0}' environment variable must be a string or be convertible to a string.
Either make sure {0} is a string, or add a 'to_string' entry for it in ENV_CONVERSIONS."#
        )
    )]
    EnvVarNotAString(String, #[label("value not representable as a string")] Span),

    #[error("Division by zero.")]
    #[diagnostic(code(nu::shell::division_by_zero), url(docsrs))]
    DivisionByZero(#[label("division by zero")] Span),

    #[error("Can't convert range to countable values")]
    #[diagnostic(code(nu::shell::range_to_countable), url(docsrs))]
    CannotCreateRange(#[label = "can't convert to countable values"] Span),

    #[error("Row number too large (max: {0}).")]
    #[diagnostic(code(nu::shell::access_beyond_end), url(docsrs))]
    AccessBeyondEnd(usize, #[label = "too large"] Span),

    #[error("Row number too large.")]
    #[diagnostic(code(nu::shell::access_beyond_end_of_stream), url(docsrs))]
    AccessBeyondEndOfStream(#[label = "too large"] Span),

    #[error("Data cannot be accessed with a cell path")]
    #[diagnostic(code(nu::shell::incompatible_path_access), url(docsrs))]
    IncompatiblePathAccess(String, #[label("{0} doesn't support cell paths")] Span),

    #[error("Cannot find column")]
    #[diagnostic(code(nu::shell::column_not_found), url(docsrs))]
    CantFindColumn(
        #[label = "cannot find column"] Span,
        #[label = "value originates here"] Span,
    ),

    #[error("Not a list value")]
    #[diagnostic(code(nu::shell::not_a_list), url(docsrs))]
    NotAList(
        #[label = "value not a list"] Span,
        #[label = "value originates here"] Span,
    ),

    #[error("External command")]
    #[diagnostic(code(nu::shell::external_command), url(docsrs), help("{1}"))]
    ExternalCommand(String, String, #[label("{0}")] Span),

    #[error("Unsupported input")]
    #[diagnostic(code(nu::shell::unsupported_input), url(docsrs))]
    UnsupportedInput(String, #[label("{0}")] Span),

    #[error("Unable to parse datetime")]
    #[diagnostic(
        code(nu::shell::datetime_parse_error),
        url(docsrs),
        help(
            r#"Examples of supported inputs:
 * "5 pm"
 * "2020/12/4"
 * "2020.12.04 22:10 +2"
 * "2020-04-12 22:10:57 +02:00"
 * "2020-04-12T22:10:57.213231+02:00"
 * "Tue, 1 Jul 2003 10:52:37 +0200""#
        )
    )]
    DatetimeParseError(#[label("datetime parsing failed")] Span),

    #[error("Network failure")]
    #[diagnostic(code(nu::shell::network_failure), url(docsrs))]
    NetworkFailure(String, #[label("{0}")] Span),

    #[error("Command not found")]
    #[diagnostic(code(nu::shell::command_not_found), url(docsrs))]
    CommandNotFound(#[label("command not found")] Span),

    #[error("Flag not found")]
    #[diagnostic(code(nu::shell::flag_not_found), url(docsrs))]
    FlagNotFound(String, #[label("{0} not found")] Span),

    #[error("File not found")]
    #[diagnostic(code(nu::shell::file_not_found), url(docsrs))]
    FileNotFound(#[label("file not found")] Span),

    #[error("File not found")]
    #[diagnostic(code(nu::shell::file_not_found), url(docsrs))]
    FileNotFoundCustom(String, #[label("{0}")] Span),

    #[error("Plugin failed to load: {0}")]
    #[diagnostic(code(nu::shell::plugin_failed_to_load), url(docsrs))]
    PluginFailedToLoad(String),

    #[error("Plugin failed to encode: {0}")]
    #[diagnostic(code(nu::shell::plugin_failed_to_encode), url(docsrs))]
    PluginFailedToEncode(String),

    #[error("Plugin failed to decode: {0}")]
    #[diagnostic(code(nu::shell::plugin_failed_to_decode), url(docsrs))]
    PluginFailedToDecode(String),

    #[error("I/O error")]
    #[diagnostic(code(nu::shell::io_error), url(docsrs), help("{0}"))]
    IOError(String),

    #[error("Cannot change to directory")]
    #[diagnostic(code(nu::shell::cannot_cd_to_directory), url(docsrs))]
    NotADirectory(#[label("is not a directory")] Span),

    #[error("Directory not found")]
    #[diagnostic(code(nu::shell::directory_not_found), url(docsrs))]
    DirectoryNotFound(#[label("directory not found")] Span),

    #[error("Directory not found")]
    #[diagnostic(code(nu::shell::directory_not_found_custom), url(docsrs))]
    DirectoryNotFoundCustom(String, #[label("{0}")] Span),

    #[error("Directory not found")]
    #[diagnostic(code(nu::shell::directory_not_found_help), url(docsrs), help("{1}"))]
    DirectoryNotFoundHelp(#[label("directory not found")] Span, String),

    #[error("Move not possible")]
    #[diagnostic(code(nu::shell::move_not_possible), url(docsrs))]
    MoveNotPossible {
        source_message: String,
        #[label("{source_message}")]
        source_span: Span,
        destination_message: String,
        #[label("{destination_message}")]
        destination_span: Span,
    },

    #[error("Move not possible")]
    #[diagnostic(code(nu::shell::move_not_possible_single), url(docsrs))]
    MoveNotPossibleSingle(String, #[label("{0}")] Span),

    #[error("Create not possible")]
    #[diagnostic(code(nu::shell::create_not_possible), url(docsrs))]
    CreateNotPossible(String, #[label("{0}")] Span),

    #[error("Remove not possible")]
    #[diagnostic(code(nu::shell::remove_not_possible), url(docsrs))]
    RemoveNotPossible(String, #[label("{0}")] Span),

    #[error("No file to be removed")]
    NoFileToBeRemoved(),
    #[error("No file to be moved")]
    NoFileToBeMoved(),
    #[error("No file to be copied")]
    NoFileToBeCopied(),

    #[error("Name not found")]
    #[diagnostic(code(nu::shell::name_not_found), url(docsrs))]
    DidYouMean(String, #[label("did you mean '{0}'?")] Span),

    #[error("Non-UTF8 string")]
    #[diagnostic(code(nu::parser::non_utf8), url(docsrs))]
    NonUtf8(#[label = "non-UTF8 string"] Span),

    #[error("Casting error")]
    #[diagnostic(code(nu::shell::downcast_not_possible), url(docsrs))]
    DowncastNotPossible(String, #[label("{0}")] Span),

    #[error("Unsupported config value")]
    #[diagnostic(code(nu::shell::unsupported_config_value), url(docsrs))]
    UnsupportedConfigValue(String, String, #[label = "expected {0}, got {1}"] Span),

    #[error("Missing config value")]
    #[diagnostic(code(nu::shell::missing_config_value), url(docsrs))]
    MissingConfigValue(String, #[label = "missing {0}"] Span),

    #[error("Negative value passed when positive one is required")]
    #[diagnostic(code(nu::shell::needs_positive_value), url(docsrs))]
    NeedsPositiveValue(#[label = "use a positive value"] Span),

    #[error("{0}")]
    #[diagnostic()]
    SpannedLabeledError(String, String, #[label("{1}")] Span),

    #[error("{0}")]
    #[diagnostic(help("{3}"))]
    SpannedLabeledErrorHelp(String, String, #[label("{1}")] Span, String),

    #[error("{0}")]
    #[diagnostic()]
    SpannedLabeledErrorRelated(
        String,
        String,
        #[label("{1}")] Span,
        #[related] Vec<ShellError>,
    ),

    #[error("{0}")]
    #[diagnostic(help("{1}"))]
    LabeledError(String, String),

    #[error("{1}")]
    #[diagnostic()]
    OutsideSpannedLabeledError(#[source_code] String, String, String, #[label("{2}")] Span),

    #[error("Deprecated command {0}")]
    #[diagnostic(code(nu::shell::deprecated_command), url(docsrs))]
    DeprecatedCommand(
        String,
        String,
        #[label = "'{0}' is deprecated. Please use '{1}' instead."] Span,
    ),
}

impl From<std::io::Error> for ShellError {
    fn from(input: std::io::Error) -> ShellError {
        ShellError::IOError(format!("{:?}", input))
    }
}

impl std::convert::From<Box<dyn std::error::Error>> for ShellError {
    fn from(input: Box<dyn std::error::Error>) -> ShellError {
        ShellError::IOError(input.to_string())
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for ShellError {
    fn from(input: Box<dyn std::error::Error + Send + Sync>) -> ShellError {
        ShellError::IOError(format!("{:?}", input))
    }
}

pub fn did_you_mean(possibilities: &[String], tried: &str) -> Option<String> {
    let mut possible_matches: Vec<_> = possibilities
        .iter()
        .map(|word| {
            let edit_distance = levenshtein_distance(word, tried);
            (edit_distance, word.to_owned())
        })
        .collect();

    possible_matches.sort();

    if let Some((_, first)) = possible_matches.into_iter().next() {
        Some(first)
    } else {
        None
    }
}

// Borrowed from here https://github.com/wooorm/levenshtein-rs
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let mut result = 0;

    /* Shortcut optimizations / degenerate cases. */
    if a == b {
        return result;
    }

    let length_a = a.chars().count();
    let length_b = b.chars().count();

    if length_a == 0 {
        return length_b;
    }

    if length_b == 0 {
        return length_a;
    }

    /* Initialize the vector.
     *
     * This is why it’s fast, normally a matrix is used,
     * here we use a single vector. */
    let mut cache: Vec<usize> = (1..).take(length_a).collect();
    let mut distance_a;
    let mut distance_b;

    /* Loop. */
    for (index_b, code_b) in b.chars().enumerate() {
        result = index_b;
        distance_a = index_b;

        for (index_a, code_a) in a.chars().enumerate() {
            distance_b = if code_a == code_b {
                distance_a
            } else {
                distance_a + 1
            };

            distance_a = cache[index_a];

            result = if distance_a > result {
                if distance_b > result {
                    result + 1
                } else {
                    distance_b
                }
            } else if distance_b > distance_a {
                distance_a + 1
            } else {
                distance_b
            };

            cache[index_a] = result;
        }
    }

    result
}
