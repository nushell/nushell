use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{ast::Operator, Span, Type};

/// The fundamental error type for the evaluation engine. These cases represent different kinds of errors
/// the evaluator might face, along with helpful spans to label. An error renderer will take this error value
/// and pass it into an error viewer to display to the user.
#[derive(Debug, Clone, Error, Diagnostic, Serialize, Deserialize)]
pub enum ShellError {
    /// An operator received two arguments of incompatible types.
    ///
    /// ## Resolution
    ///
    /// Check each argument's type and convert one or both as needed.
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

    /// An arithmetic operation's resulting value overflowed its possible size.
    ///
    /// ## Resolution
    ///
    /// Check the inputs to the operation and add guards for their sizes.
    /// Integers are generally of size i64, floats are generally f64.
    #[error("Operator overflow.")]
    #[diagnostic(code(nu::shell::operator_overflow), url(docsrs))]
    OperatorOverflow(String, #[label = "{0}"] Span),

    /// The pipelined input into a command was not of the expected type. For example, it might
    /// expect a string input, but received a table instead.
    ///
    /// ## Resolution
    ///
    /// Check the relevant pipeline and extract or convert values as needed.
    #[error("Pipeline mismatch.")]
    #[diagnostic(code(nu::shell::pipeline_mismatch), url(docsrs))]
    PipelineMismatch(
        String,
        #[label("expected: {0}")] Span,
        #[label("value originates from here")] Span,
    ),

    /// A command received an argument of the wrong type.
    ///
    /// ## Resolution
    ///
    /// Convert the argument type before passing it in, or change the command to accept the type.
    #[error("Type mismatch")]
    #[diagnostic(code(nu::shell::type_mismatch), url(docsrs))]
    TypeMismatch(String, #[label = "needs {0}"] Span),

    /// This value cannot be used with this operator.
    ///
    /// ## Resolution
    ///
    /// Not all values, for example custom values, can be used with all operators. Either
    /// implement support for the operator on this type, or convert the type to a supported one.
    #[error("Unsupported operator: {0}.")]
    #[diagnostic(code(nu::shell::unsupported_operator), url(docsrs))]
    UnsupportedOperator(Operator, #[label = "unsupported operator"] Span),

    /// An operator was not recognized during evaluation.
    ///
    /// ## Resolution
    ///
    /// Did you write the correct operator?
    #[error("Unknown operator: {0}.")]
    #[diagnostic(code(nu::shell::unknown_operator), url(docsrs))]
    UnknownOperator(String, #[label = "unknown operator"] Span),

    /// An expected command parameter is missing.
    ///
    /// ## Resolution
    ///
    /// Add the expected parameter and try again.
    #[error("Missing parameter: {0}.")]
    #[diagnostic(code(nu::shell::missing_parameter), url(docsrs))]
    MissingParameter(String, #[label = "missing parameter: {0}"] Span),

    /// Two parameters conflict with each other or are otherwise mutually exclusive.
    ///
    /// ## Resolution
    ///
    /// Remove one of the parameters/options and try again.
    #[error("Incompatible parameters.")]
    #[diagnostic(code(nu::shell::incompatible_parameters), url(docsrs))]
    IncompatibleParameters {
        left_message: String,
        // Be cautious, as flags can share the same span, resulting in a panic (ex: `rm -pt`)
        #[label("{left_message}")]
        left_span: Span,
        right_message: String,
        #[label("{right_message}")]
        right_span: Span,
    },

    /// There's some issue with number or matching of delimiters in an expression.
    ///
    /// ## Resolution
    ///
    /// Check your syntax for mismatched braces, RegExp syntax errors, etc, based on the specific error message.
    #[error("Delimiter error")]
    #[diagnostic(code(nu::shell::delimiter_error), url(docsrs))]
    DelimiterError(String, #[label("{0}")] Span),

    /// An operation received parameters with some sort of incompatibility
    /// (for example, different number of rows in a table, incompatible column names, etc).
    ///
    /// ## Resolution
    ///
    /// Refer to the specific error message for details on what's incompatible and then fix your
    /// inputs to make sure they match that way.
    #[error("Incompatible parameters.")]
    #[diagnostic(code(nu::shell::incompatible_parameters), url(docsrs))]
    IncompatibleParametersSingle(String, #[label = "{0}"] Span),

    /// This build of nushell implements this feature, but it has not been enabled.
    ///
    /// ## Resolution
    ///
    /// Rebuild nushell with the appropriate feature enabled.
    #[error("Feature not enabled.")]
    #[diagnostic(code(nu::shell::feature_not_enabled), url(docsrs))]
    FeatureNotEnabled(#[label = "feature not enabled"] Span),

    /// You're trying to run an unsupported external command.
    ///
    /// ## Resolution
    ///
    /// Make sure there's an appropriate `run-external` declaration for this external command.
    #[error("Running external commands not supported")]
    #[diagnostic(code(nu::shell::external_commands), url(docsrs))]
    ExternalNotSupported(#[label = "external not supported"] Span),

    /// The given probability input is invalid. The probability must be between 0 and 1.
    ///
    /// ## Resolution
    ///
    /// Make sure the probability is between 0 and 1 and try again.
    #[error("Invalid Probability.")]
    #[diagnostic(code(nu::shell::invalid_probability), url(docsrs))]
    InvalidProbability(#[label = "invalid probability"] Span),

    /// The first value in a `..` range must be compatible with the second one.
    ///
    /// ## Resolution
    ///
    /// Check to make sure both values are compatible, and that the values are enumerable in Nushell.
    #[error("Invalid range {0}..{1}")]
    #[diagnostic(code(nu::shell::invalid_range), url(docsrs))]
    InvalidRange(String, String, #[label = "expected a valid range"] Span),

    /// Catastrophic nushell failure. This reflects a completely unexpected or unrecoverable error.
    ///
    /// ## Resolution
    ///
    /// It is very likely that this is a bug. Please file an issue at https://github.com/nushell/nushell/issues with relevant information.
    #[error("Nushell failed: {0}.")]
    #[diagnostic(code(nu::shell::nushell_failed), url(docsrs))]
    // Only use this one if Nushell completely falls over and hits a state that isn't possible or isn't recoverable
    NushellFailed(String),

    /// Catastrophic nushell failure. This reflects a completely unexpected or unrecoverable error.
    ///
    /// ## Resolution
    ///
    /// It is very likely that this is a bug. Please file an issue at https://github.com/nushell/nushell/issues with relevant information.
    #[error("Nushell failed: {0}.")]
    #[diagnostic(code(nu::shell::nushell_failed_spanned), url(docsrs))]
    // Only use this one if Nushell completely falls over and hits a state that isn't possible or isn't recoverable
    NushellFailedSpanned(String, String, #[label = "{1}"] Span),

    /// Catastrophic nushell failure. This reflects a completely unexpected or unrecoverable error.
    ///
    /// ## Resolution
    ///
    /// It is very likely that this is a bug. Please file an issue at https://github.com/nushell/nushell/issues with relevant information.
    #[error("Nushell failed: {0}.")]
    #[diagnostic(code(nu::shell::nushell_failed_help), url(docsrs))]
    // Only use this one if Nushell completely falls over and hits a state that isn't possible or isn't recoverable
    NushellFailedHelp(String, #[help] String),

    /// Catastrophic nushell failure. This reflects a completely unexpected or unrecoverable error.
    ///
    /// ## Resolution
    ///
    /// It is very likely that this is a bug. Please file an issue at https://github.com/nushell/nushell/issues with relevant information.
    #[error("Nushell failed: {0}.")]
    #[diagnostic(code(nu::shell::nushell_failed_spanned_help), url(docsrs))]
    // Only use this one if Nushell completely falls over and hits a state that isn't possible or isn't recoverable
    NushellFailedSpannedHelp(String, String, #[label = "{1}"] Span, #[help] String),

    /// A referenced variable was not found at runtime.
    ///
    /// ## Resolution
    ///
    /// Check the variable name. Did you typo it? Did you forget to declare it? Is the casing right?
    #[error("Variable not found")]
    #[diagnostic(code(nu::shell::variable_not_found), url(docsrs))]
    VariableNotFoundAtRuntime(#[label = "variable not found"] Span),

    /// A referenced environment variable was not found at runtime.
    ///
    /// ## Resolution
    ///
    /// Check the environment variable name. Did you typo it? Did you forget to declare it? Is the casing right?
    #[error("Environment variable '{0}' not found")]
    #[diagnostic(code(nu::shell::env_variable_not_found), url(docsrs))]
    EnvVarNotFoundAtRuntime(String, #[label = "environment variable not found"] Span),

    /// A referenced module was not found at runtime.
    ///
    /// ## Resolution
    ///
    /// Check the module name. Did you typo it? Did you forget to declare it? Is the casing right?
    #[error("Module '{0}' not found")]
    #[diagnostic(code(nu::shell::module_not_found), url(docsrs))]
    ModuleNotFoundAtRuntime(String, #[label = "module not found"] Span),

    /// A referenced module or overlay was not found at runtime.
    ///
    /// ## Resolution
    ///
    /// Check the module name. Did you typo it? Did you forget to declare it? Is the casing right?
    #[error("Module or overlay'{0}' not found")]
    #[diagnostic(code(nu::shell::module_or_overlay_not_found), url(docsrs))]
    ModuleOrOverlayNotFoundAtRuntime(String, #[label = "not a module or overlay"] Span),

    /// A referenced overlay was not found at runtime.
    ///
    /// ## Resolution
    ///
    /// Check the overlay name. Did you typo it? Did you forget to declare it? Is the casing right?
    #[error("Overlay '{0}' not found")]
    #[diagnostic(code(nu::shell::overlay_not_found), url(docsrs))]
    OverlayNotFoundAtRuntime(String, #[label = "overlay not found"] Span),

    /// The given item was not found. This is a fairly generic error that depends on context.
    ///
    /// ## Resolution
    ///
    /// This error is triggered in various places, and simply signals that "something" was not found. Refer to the specific error message for further details.
    #[error("Not found.")]
    #[diagnostic(code(nu::parser::not_found), url(docsrs))]
    NotFound(#[label = "did not find anything under this name"] Span),

    /// Failed to convert a value of one type into a different type.
    ///
    /// ## Resolution
    ///
    /// Not all values can be coerced this way. Check the supported type(s) and try again.
    #[error("Can't convert to {0}.")]
    #[diagnostic(code(nu::shell::cant_convert), url(docsrs))]
    CantConvert(
        String,
        String,
        #[label("can't convert {1} to {0}")] Span,
        #[help] Option<String>,
    ),

    /// Failed to convert a value of one type into a different type. Includes hint for what the first value is.
    ///
    /// ## Resolution
    ///
    /// Not all values can be coerced this way. Check the supported type(s) and try again.
    #[error("Can't convert {1} `{2}` to {0}.")]
    #[diagnostic(code(nu::shell::cant_convert_with_value), url(docsrs))]
    CantConvertWithValue(
        String,
        String,
        String,
        #[label("can't be converted to {0}")] Span,
        #[label("this {1} value...")] Span,
        #[help] Option<String>,
    ),

    /// An environment variable cannot be represented as a string.
    ///
    /// ## Resolution
    ///
    /// Not all types can be converted to environment variable values, which must be strings. Check the input type and try again.
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

    /// Division by zero is not a thing.
    ///
    /// ## Resolution
    ///
    /// Add a guard of some sort to check whether a denominator input to this division is zero, and branch off if that's the case.
    #[error("Division by zero.")]
    #[diagnostic(code(nu::shell::division_by_zero), url(docsrs))]
    DivisionByZero(#[label("division by zero")] Span),

    /// An error happened while tryin to create a range.
    ///
    /// This can happen in various unexpected situations, for example if the range would loop forever (as would be the case with a 0-increment).
    ///
    /// ## Resolution
    ///
    /// Check your range values to make sure they're countable and would not loop forever.
    #[error("Can't convert range to countable values")]
    #[diagnostic(code(nu::shell::range_to_countable), url(docsrs))]
    CannotCreateRange(#[label = "can't convert to countable values"] Span),

    /// You attempted to access an index beyond the available length of a value.
    ///
    /// ## Resolution
    ///
    /// Check your lengths and try again.
    #[error("Row number too large (max: {0}).")]
    #[diagnostic(code(nu::shell::access_beyond_end), url(docsrs))]
    AccessBeyondEnd(usize, #[label = "index too large (max: {0})"] Span),

    /// You attempted to access an index beyond the available length of a stream.
    ///
    /// ## Resolution
    ///
    /// Check your lengths and try again.
    #[error("Row number too large.")]
    #[diagnostic(code(nu::shell::access_beyond_end_of_stream), url(docsrs))]
    AccessBeyondEndOfStream(#[label = "index too large"] Span),

    /// Tried to index into a type that does not support pathed access.
    ///
    /// ## Resolution
    ///
    /// Check your types. Only composite types can be pathed into.
    #[error("Data cannot be accessed with a cell path")]
    #[diagnostic(code(nu::shell::incompatible_path_access), url(docsrs))]
    IncompatiblePathAccess(String, #[label("{0} doesn't support cell paths")] Span),

    /// The requested column does not exist.
    ///
    /// ## Resolution
    ///
    /// Check the spelling of your column name. Did you forget to rename a column somewhere?
    #[error("Cannot find column")]
    #[diagnostic(code(nu::shell::column_not_found), url(docsrs))]
    CantFindColumn(
        #[label = "cannot find column"] Span,
        #[label = "value originates here"] Span,
    ),

    /// Attempted to insert a column into a table, but a column with that name already exists.
    ///
    /// ## Resolution
    ///
    /// Drop or rename the existing column (check `rename -h`) and try again.
    #[error("Column already exists")]
    #[diagnostic(code(nu::shell::column_already_exists), url(docsrs))]
    ColumnAlreadyExists(
        #[label = "column already exists"] Span,
        #[label = "value originates here"] Span,
    ),

    /// The given operation can only be performed on lists.
    ///
    /// ## Resolution
    ///
    /// Check the input type to this command. Are you sure it's a list?
    #[error("Not a list value")]
    #[diagnostic(code(nu::shell::not_a_list), url(docsrs))]
    NotAList(
        #[label = "value not a list"] Span,
        #[label = "value originates here"] Span,
    ),

    /// An error happened while performing an external command.
    ///
    /// ## Resolution
    ///
    /// This error is fairly generic. Refer to the specific error message for further details.
    #[error("External command failed")]
    #[diagnostic(code(nu::shell::external_command), url(docsrs), help("{1}"))]
    ExternalCommand(String, String, #[label("{0}")] Span),

    /// An operation was attempted with an input unsupported for some reason.
    ///
    /// ## Resolution
    ///
    /// This error is fairly generic. Refer to the specific error message for further details.
    #[error("Unsupported input")]
    #[diagnostic(code(nu::shell::unsupported_input), url(docsrs))]
    UnsupportedInput(String, #[label("{0}")] Span),

    /// Failed to parse an input into a datetime value.
    ///
    /// ## Resolution
    ///
    /// Make sure your datetime input format is correct.
    ///
    /// For example, these are some valid formats:
    ///
    /// * "5 pm"
    /// * "2020/12/4"
    /// * "2020.12.04 22:10 +2"
    /// * "2020-04-12 22:10:57 +02:00"
    /// * "2020-04-12T22:10:57.213231+02:00"
    /// * "Tue, 1 Jul 2003 10:52:37 +0200""#
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

    /// A network operation failed.
    ///
    /// ## Resolution
    ///
    /// It's always DNS.
    #[error("Network failure")]
    #[diagnostic(code(nu::shell::network_failure), url(docsrs))]
    NetworkFailure(String, #[label("{0}")] Span),

    /// Help text for this command could not be found.
    ///
    /// ## Resolution
    ///
    /// Check the spelling for the requested command and try again. Are you sure it's defined and your configurations are loading correctly? Can you execute it?
    #[error("Command not found")]
    #[diagnostic(code(nu::shell::command_not_found), url(docsrs))]
    CommandNotFound(#[label("command not found")] Span),

    /// A flag was not found.
    #[error("Flag not found")]
    #[diagnostic(code(nu::shell::flag_not_found), url(docsrs))]
    // NOTE: Seems to be unused. Removable?
    FlagNotFound(String, #[label("{0} not found")] Span),

    /// Failed to find a file during a nushell operation.
    ///
    /// ## Resolution
    ///
    /// Does the file in the error message exist? Is it readable and accessible? Is the casing right?
    #[error("File not found")]
    #[diagnostic(code(nu::shell::file_not_found), url(docsrs))]
    FileNotFound(#[label("file not found")] Span),

    /// Failed to find a file during a nushell operation.
    ///
    /// ## Resolution
    ///
    /// Does the file in the error message exist? Is it readable and accessible? Is the casing right?
    #[error("File not found")]
    #[diagnostic(code(nu::shell::file_not_found), url(docsrs))]
    FileNotFoundCustom(String, #[label("{0}")] Span),

    /// A plugin failed to load.
    ///
    /// ## Resolution
    ///
    /// This is a failry generic error. Refer to the specific error message for further details.
    #[error("Plugin failed to load: {0}")]
    #[diagnostic(code(nu::shell::plugin_failed_to_load), url(docsrs))]
    PluginFailedToLoad(String),

    /// A message from a plugin failed to encode.
    ///
    /// ## Resolution
    ///
    /// This is likely a bug with the plugin itself.
    #[error("Plugin failed to encode: {0}")]
    #[diagnostic(code(nu::shell::plugin_failed_to_encode), url(docsrs))]
    PluginFailedToEncode(String),

    /// A message to a plugin failed to decode.
    ///
    /// ## Resolution
    ///
    /// This is either an issue with the inputs to a plugin (bad JSON?) or a bug in the plugin itself. Fix or report as appropriate.
    #[error("Plugin failed to decode: {0}")]
    #[diagnostic(code(nu::shell::plugin_failed_to_decode), url(docsrs))]
    PluginFailedToDecode(String),

    /// An I/O operation failed.
    ///
    /// ## Resolution
    ///
    /// This is a generic error. Refer to the specific error message for further details.
    #[error("I/O error")]
    #[diagnostic(code(nu::shell::io_error), url(docsrs), help("{0}"))]
    IOError(String),

    /// Tried to `cd` to a path that isn't a directory.
    ///
    /// ## Resolution
    ///
    /// Make sure the path is a directory. It currently exists, but is of some other type, like a file.
    #[error("Cannot change to directory")]
    #[diagnostic(code(nu::shell::cannot_cd_to_directory), url(docsrs))]
    NotADirectory(#[label("is not a directory")] Span),

    /// Attempted to perform an operation on a directory that doesn't exist.
    ///
    /// ## Resolution
    ///
    /// Make sure the directory in the error message actually exists before trying again.
    #[error("Directory not found")]
    #[diagnostic(code(nu::shell::directory_not_found), url(docsrs))]
    DirectoryNotFound(#[label("directory not found")] Span, #[help] Option<String>),

    /// Attempted to perform an operation on a directory that doesn't exist.
    ///
    /// ## Resolution
    ///
    /// Make sure the directory in the error message actually exists before trying again.
    #[error("Directory not found")]
    #[diagnostic(code(nu::shell::directory_not_found_custom), url(docsrs))]
    DirectoryNotFoundCustom(String, #[label("{0}")] Span),

    /// The requested move operation cannot be completed. This is typically because both paths exist,
    /// but are of different types. For example, you might be trying to overwrite an existing file with
    /// a directory.
    ///
    /// ## Resolution
    ///
    /// Make sure the destination path does not exist before moving a directory.
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

    /// The requested move operation cannot be completed. This is typically because both paths exist,
    /// but are of different types. For example, you might be trying to overwrite an existing file with
    /// a directory.
    ///
    /// ## Resolution
    ///
    /// Make sure the destination path does not exist before moving a directory.
    #[error("Move not possible")]
    #[diagnostic(code(nu::shell::move_not_possible_single), url(docsrs))]
    // NOTE: Currently not actively used.
    MoveNotPossibleSingle(String, #[label("{0}")] Span),

    /// Failed to create either a file or directory.
    ///
    /// ## Resolution
    ///
    /// This is a fairly generic error. Refer to the specific error message for further details.
    #[error("Create not possible")]
    #[diagnostic(code(nu::shell::create_not_possible), url(docsrs))]
    CreateNotPossible(String, #[label("{0}")] Span),

    /// Changing the access time ("atime") of this file is not possible.
    ///
    /// ## Resolution
    ///
    /// This can be for various reasons, such as your platform or permission flags. Refer to the specific error message for more details.
    #[error("Not possible to change the access time")]
    #[diagnostic(code(nu::shell::change_access_time_not_possible), url(docsrs))]
    ChangeAccessTimeNotPossible(String, #[label("{0}")] Span),

    /// Changing the modification time ("mtime") of this file is not possible.
    ///
    /// ## Resolution
    ///
    /// This can be for various reasons, such as your platform or permission flags. Refer to the specific error message for more details.
    #[error("Not possible to change the modified time")]
    #[diagnostic(code(nu::shell::change_modified_time_not_possible), url(docsrs))]
    ChangeModifiedTimeNotPossible(String, #[label("{0}")] Span),

    /// Unable to remove this item.
    #[error("Remove not possible")]
    #[diagnostic(code(nu::shell::remove_not_possible), url(docsrs))]
    // NOTE: Currently unused. Remove?
    RemoveNotPossible(String, #[label("{0}")] Span),

    // These three are unused. Remove?
    #[error("No file to be removed")]
    NoFileToBeRemoved(),
    #[error("No file to be moved")]
    NoFileToBeMoved(),
    #[error("No file to be copied")]
    NoFileToBeCopied(),

    /// Error while trying to read a file
    ///
    /// ## Resolution
    ///
    /// The error will show the result from a file operation
    #[error("Error trying to read file")]
    #[diagnostic(code(nu::shell::error_reading_file), url(docsrs))]
    ReadingFile(String, #[label("{0}")] Span),

    /// A name was not found. Did you mean a different name?
    ///
    /// ## Resolution
    ///
    /// The error message will suggest a possible match for what you meant.
    #[error("Name not found")]
    #[diagnostic(code(nu::shell::name_not_found), url(docsrs))]
    DidYouMean(String, #[label("did you mean '{0}'?")] Span),

    /// The given input must be valid UTF-8 for further processing.
    ///
    /// ## Resolution
    ///
    /// Check your input's encoding. Are there any funny characters/bytes?
    #[error("Non-UTF8 string")]
    #[diagnostic(code(nu::parser::non_utf8), url(docsrs))]
    NonUtf8(#[label = "non-UTF8 string"] Span),

    /// A custom value could not be converted to a Dataframe.
    ///
    /// ## Resolution
    ///
    /// Make sure conversion to a Dataframe is possible for this value or convert it to a type that does, first.
    #[error("Casting error")]
    #[diagnostic(code(nu::shell::downcast_not_possible), url(docsrs))]
    DowncastNotPossible(String, #[label("{0}")] Span),

    /// The value given for this configuration is not supported.
    ///
    /// ## Resolution
    ///
    /// Refer to the specific error message for details and convert values as needed.
    #[error("Unsupported config value")]
    #[diagnostic(code(nu::shell::unsupported_config_value), url(docsrs))]
    UnsupportedConfigValue(String, String, #[label = "expected {0}, got {1}"] Span),

    /// An expected configuration value is not present.
    ///
    /// ## Resolution
    ///
    /// Refer to the specific error message and add the configuration value to your config file as needed.
    #[error("Missing config value")]
    #[diagnostic(code(nu::shell::missing_config_value), url(docsrs))]
    MissingConfigValue(String, #[label = "missing {0}"] Span),

    /// Negative value passed when positive ons is required.
    ///
    /// ## Resolution
    ///
    /// Guard against negative values or check your inputs.
    #[error("Negative value passed when positive one is required")]
    #[diagnostic(code(nu::shell::needs_positive_value), url(docsrs))]
    NeedsPositiveValue(#[label = "use a positive value"] Span),

    /// This is a generic error type used for different situations.
    #[error("{0}")]
    #[diagnostic()]
    GenericError(
        String,
        String,
        #[label("{1}")] Option<Span>,
        #[help] Option<String>,
        #[related] Vec<ShellError>,
    ),

    /// This is a generic error type used for different situations.
    #[error("{1}")]
    #[diagnostic()]
    OutsideSpannedLabeledError(#[source_code] String, String, String, #[label("{2}")] Span),

    /// Attempted to use a deprecated command.
    ///
    /// ## Resolution
    ///
    /// Check the help for the new suggested command and update your script accordingly.
    #[error("Deprecated command {0}")]
    #[diagnostic(code(nu::shell::deprecated_command), url(docsrs))]
    DeprecatedCommand(
        String,
        String,
        #[label = "'{0}' is deprecated. Please use '{1}' instead."] Span,
    ),

    /// Non-Unicode input received.
    ///
    /// ## Resolution
    ///
    /// Check that your path is UTF-8 compatible.
    #[error("Non-Unicode input received.")]
    #[diagnostic(code(nu::shell::non_unicode_input), url(docsrs))]
    NonUnicodeInput,

    // /// Path not found.
    // #[error("Path not found.")]
    // PathNotFound,
    /// Unexpected abbr component.
    ///
    /// ## Resolution
    ///
    /// Check the path abbreviation to ensure that it is valid.
    #[error("Unexpected abbr component `{0}`.")]
    #[diagnostic(code(nu::shell::unexpected_path_abbreviateion), url(docsrs))]
    UnexpectedAbbrComponent(String),
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

pub fn into_code(err: &ShellError) -> Option<String> {
    err.code().map(|code| code.to_string())
}

pub fn did_you_mean(possibilities: &[String], tried: &str) -> Option<String> {
    let mut possible_matches: Vec<_> = possibilities
        .iter()
        .map(|word| {
            let edit_distance = levenshtein_distance(&word.to_lowercase(), &tried.to_lowercase());
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

#[cfg(test)]
mod tests {
    use super::did_you_mean;

    #[test]
    fn did_you_mean_works_with_wrong_case() {
        let possibilities = &["OS".into(), "PWD".into()];
        let actual = did_you_mean(possibilities, "pwd");
        let expected = Some(String::from("PWD"));
        assert_eq!(actual, expected)
    }

    #[test]
    fn did_you_mean_works_with_typo() {
        let possibilities = &["OS".into(), "PWD".into()];
        let actual = did_you_mean(possibilities, "PWF");
        let expected = Some(String::from("PWD"));
        assert_eq!(actual, expected)
    }

    #[test]
    fn did_you_mean_works_with_wrong_case_and_typo() {
        let possibilities = &["OS".into(), "PWD".into()];
        let actual = did_you_mean(possibilities, "pwf");
        let expected = Some(String::from("PWD"));
        assert_eq!(actual, expected)
    }
}
