use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{ast::Operator, Span, Type, Value};

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
    #[diagnostic(code(nu::shell::type_mismatch))]
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
    #[diagnostic(code(nu::shell::operator_overflow), help("{help}"))]
    OperatorOverflow {
        msg: String,
        #[label = "{msg}"]
        span: Span,
        help: String,
    },

    /// The pipelined input into a command was not of the expected type. For example, it might
    /// expect a string input, but received a table instead.
    ///
    /// ## Resolution
    ///
    /// Check the relevant pipeline and extract or convert values as needed.
    #[error("Pipeline mismatch.")]
    #[diagnostic(code(nu::shell::pipeline_mismatch))]
    PipelineMismatch {
        exp_input_type: String,
        #[label("expected: {exp_input_type}")]
        dst_span: Span,
        #[label("value originates from here")]
        src_span: Span,
    },

    // TODO: properly unify
    /// The pipelined input into a command was not of the expected type. For example, it might
    /// expect a string input, but received a table instead.
    ///
    /// (duplicate of [`ShellError::PipelineMismatch`] that reports the observed type)
    ///
    /// ## Resolution
    ///
    /// Check the relevant pipeline and extract or convert values as needed.
    #[error("Input type not supported.")]
    #[diagnostic(code(nu::shell::only_supports_this_input_type))]
    OnlySupportsThisInputType {
        exp_input_type: String,
        wrong_type: String,
        #[label("only {exp_input_type} input data is supported")]
        dst_span: Span,
        #[label("input type: {wrong_type}")]
        src_span: Span,
    },

    /// No input value was piped into the command.
    ///
    /// ## Resolution
    ///
    /// Only use this command to process values from a previous expression.
    #[error("Pipeline empty.")]
    #[diagnostic(code(nu::shell::pipeline_mismatch))]
    PipelineEmpty {
        #[label("no input value was piped in")]
        dst_span: Span,
    },

    // TODO: remove non type error usages
    /// A command received an argument of the wrong type.
    ///
    /// ## Resolution
    ///
    /// Convert the argument type before passing it in, or change the command to accept the type.
    #[error("Type mismatch.")]
    #[diagnostic(code(nu::shell::type_mismatch))]
    TypeMismatch {
        err_message: String,
        #[label = "{err_message}"]
        span: Span,
    },

    /// A command received an argument with correct type but incorrect value.
    ///
    /// ## Resolution
    ///
    /// Correct the argument value before passing it in or change the command.
    #[error("Incorrect value.")]
    #[diagnostic(code(nu::shell::incorrect_value))]
    IncorrectValue {
        msg: String,
        #[label = "{msg}"]
        span: Span,
    },

    /// This value cannot be used with this operator.
    ///
    /// ## Resolution
    ///
    /// Not all values, for example custom values, can be used with all operators. Either
    /// implement support for the operator on this type, or convert the type to a supported one.
    #[error("Unsupported operator: {operator}.")]
    #[diagnostic(code(nu::shell::unsupported_operator))]
    UnsupportedOperator {
        operator: Operator,
        #[label = "unsupported operator"]
        span: Span,
    },

    /// Invalid assignment left-hand side
    ///
    /// ## Resolution
    ///
    /// Assignment requires that you assign to a variable or variable cell path.
    #[error("Assignment operations require a variable.")]
    #[diagnostic(code(nu::shell::assignment_requires_variable))]
    AssignmentRequiresVar {
        #[label = "needs to be a variable"]
        lhs_span: Span,
    },

    /// Invalid assignment left-hand side
    ///
    /// ## Resolution
    ///
    /// Assignment requires that you assign to a mutable variable or cell path.
    #[error("Assignment to an immutable variable.")]
    #[diagnostic(code(nu::shell::assignment_requires_mutable_variable))]
    AssignmentRequiresMutableVar {
        #[label = "needs to be a mutable variable"]
        lhs_span: Span,
    },

    /// An operator was not recognized during evaluation.
    ///
    /// ## Resolution
    ///
    /// Did you write the correct operator?
    #[error("Unknown operator: {op_token}.")]
    #[diagnostic(code(nu::shell::unknown_operator))]
    UnknownOperator {
        op_token: String,
        #[label = "unknown operator"]
        span: Span,
    },

    /// An expected command parameter is missing.
    ///
    /// ## Resolution
    ///
    /// Add the expected parameter and try again.
    #[error("Missing parameter: {param_name}.")]
    #[diagnostic(code(nu::shell::missing_parameter))]
    MissingParameter {
        param_name: String,
        #[label = "missing parameter: {param_name}"]
        span: Span,
    },

    /// Two parameters conflict with each other or are otherwise mutually exclusive.
    ///
    /// ## Resolution
    ///
    /// Remove one of the parameters/options and try again.
    #[error("Incompatible parameters.")]
    #[diagnostic(code(nu::shell::incompatible_parameters))]
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
    #[diagnostic(code(nu::shell::delimiter_error))]
    DelimiterError {
        msg: String,
        #[label("{msg}")]
        span: Span,
    },

    /// An operation received parameters with some sort of incompatibility
    /// (for example, different number of rows in a table, incompatible column names, etc).
    ///
    /// ## Resolution
    ///
    /// Refer to the specific error message for details on what's incompatible and then fix your
    /// inputs to make sure they match that way.
    #[error("Incompatible parameters.")]
    #[diagnostic(code(nu::shell::incompatible_parameters))]
    IncompatibleParametersSingle {
        msg: String,
        #[label = "{msg}"]
        span: Span,
    },

    /// You're trying to run an unsupported external command.
    ///
    /// ## Resolution
    ///
    /// Make sure there's an appropriate `run-external` declaration for this external command.
    #[error("Running external commands not supported")]
    #[diagnostic(code(nu::shell::external_commands))]
    ExternalNotSupported {
        #[label = "external not supported"]
        span: Span,
    },

    // TODO: consider moving to a more generic error variant for invalid values
    /// The given probability input is invalid. The probability must be between 0 and 1.
    ///
    /// ## Resolution
    ///
    /// Make sure the probability is between 0 and 1 and try again.
    #[error("Invalid Probability.")]
    #[diagnostic(code(nu::shell::invalid_probability))]
    InvalidProbability {
        #[label = "invalid probability: must be between 0 and 1"]
        span: Span,
    },

    /// The first value in a `..` range must be compatible with the second one.
    ///
    /// ## Resolution
    ///
    /// Check to make sure both values are compatible, and that the values are enumerable in Nushell.
    #[error("Invalid range {left_flank}..{right_flank}")]
    #[diagnostic(code(nu::shell::invalid_range))]
    InvalidRange {
        left_flank: String,
        right_flank: String,
        #[label = "expected a valid range"]
        span: Span,
    },

    /// Catastrophic nushell failure. This reflects a completely unexpected or unrecoverable error.
    ///
    /// ## Resolution
    ///
    /// It is very likely that this is a bug. Please file an issue at https://github.com/nushell/nushell/issues with relevant information.
    #[error("Nushell failed: {msg}.")]
    #[diagnostic(
        code(nu::shell::nushell_failed),
        help(
        "This shouldn't happen. Please file an issue: https://github.com/nushell/nushell/issues"
    ))]
    // Only use this one if Nushell completely falls over and hits a state that isn't possible or isn't recoverable
    NushellFailed { msg: String },

    /// Catastrophic nushell failure. This reflects a completely unexpected or unrecoverable error.
    ///
    /// ## Resolution
    ///
    /// It is very likely that this is a bug. Please file an issue at https://github.com/nushell/nushell/issues with relevant information.
    #[error("Nushell failed: {msg}.")]
    #[diagnostic(
        code(nu::shell::nushell_failed_spanned),
        help(
        "This shouldn't happen. Please file an issue: https://github.com/nushell/nushell/issues"
    ))]
    // Only use this one if Nushell completely falls over and hits a state that isn't possible or isn't recoverable
    NushellFailedSpanned {
        msg: String,
        label: String,
        #[label = "{label}"]
        span: Span,
    },

    /// Catastrophic nushell failure. This reflects a completely unexpected or unrecoverable error.
    ///
    /// ## Resolution
    ///
    /// It is very likely that this is a bug. Please file an issue at https://github.com/nushell/nushell/issues with relevant information.
    #[error("Nushell failed: {msg}.")]
    #[diagnostic(code(nu::shell::nushell_failed_help))]
    // Only use this one if Nushell completely falls over and hits a state that isn't possible or isn't recoverable
    NushellFailedHelp {
        msg: String,
        #[help]
        help: String,
    },

    /// A referenced variable was not found at runtime.
    ///
    /// ## Resolution
    ///
    /// Check the variable name. Did you typo it? Did you forget to declare it? Is the casing right?
    #[error("Variable not found")]
    #[diagnostic(code(nu::shell::variable_not_found))]
    VariableNotFoundAtRuntime {
        #[label = "variable not found"]
        span: Span,
    },

    /// A referenced environment variable was not found at runtime.
    ///
    /// ## Resolution
    ///
    /// Check the environment variable name. Did you typo it? Did you forget to declare it? Is the casing right?
    #[error("Environment variable '{envvar_name}' not found")]
    #[diagnostic(code(nu::shell::env_variable_not_found))]
    EnvVarNotFoundAtRuntime {
        envvar_name: String,
        #[label = "environment variable not found"]
        span: Span,
    },

    /// A referenced module was not found at runtime.
    ///
    /// ## Resolution
    ///
    /// Check the module name. Did you typo it? Did you forget to declare it? Is the casing right?
    #[error("Module '{mod_name}' not found")]
    #[diagnostic(code(nu::shell::module_not_found))]
    ModuleNotFoundAtRuntime {
        mod_name: String,
        #[label = "module not found"]
        span: Span,
    },

    /// A referenced overlay was not found at runtime.
    ///
    /// ## Resolution
    ///
    /// Check the overlay name. Did you typo it? Did you forget to declare it? Is the casing right?
    #[error("Overlay '{overlay_name}' not found")]
    #[diagnostic(code(nu::shell::overlay_not_found))]
    OverlayNotFoundAtRuntime {
        overlay_name: String,
        #[label = "overlay not found"]
        span: Span,
    },

    /// The given item was not found. This is a fairly generic error that depends on context.
    ///
    /// ## Resolution
    ///
    /// This error is triggered in various places, and simply signals that "something" was not found. Refer to the specific error message for further details.
    #[error("Not found.")]
    #[diagnostic(code(nu::parser::not_found))]
    NotFound {
        #[label = "did not find anything under this name"]
        span: Span,
    },

    /// Failed to convert a value of one type into a different type.
    ///
    /// ## Resolution
    ///
    /// Not all values can be coerced this way. Check the supported type(s) and try again.
    #[error("Can't convert to {to_type}.")]
    #[diagnostic(code(nu::shell::cant_convert))]
    CantConvert {
        to_type: String,
        from_type: String,
        #[label("can't convert {from_type} to {to_type}")]
        span: Span,
        #[help]
        help: Option<String>,
    },

    /// Failed to convert a value of one type into a different type. Includes hint for what the first value is.
    ///
    /// ## Resolution
    ///
    /// Not all values can be coerced this way. Check the supported type(s) and try again.
    #[error("Can't convert {from_type} `{details}` to {to_type}.")]
    #[diagnostic(code(nu::shell::cant_convert_with_value))]
    CantConvertWithValue {
        to_type: String,
        from_type: String,
        details: String,
        #[label("can't be converted to {to_type}")]
        dst_span: Span,
        #[label("this {from_type} value...")]
        src_span: Span,
        #[help]
        help: Option<String>,
    },

    /// An environment variable cannot be represented as a string.
    ///
    /// ## Resolution
    ///
    /// Not all types can be converted to environment variable values, which must be strings. Check the input type and try again.
    #[error("'{envvar_name}' is not representable as a string.")]
    #[diagnostic(
            code(nu::shell::env_var_not_a_string),
            help(
                r#"The '{envvar_name}' environment variable must be a string or be convertible to a string.
    Either make sure '{envvar_name}' is a string, or add a 'to_string' entry for it in ENV_CONVERSIONS."#
            )
        )]
    EnvVarNotAString {
        envvar_name: String,
        #[label("value not representable as a string")]
        span: Span,
    },

    /// This environment variable cannot be set manually.
    ///
    /// ## Resolution
    ///
    /// This environment variable is set automatically by Nushell and cannot not be set manually.
    #[error("{envvar_name} cannot be set manually.")]
    #[diagnostic(
        code(nu::shell::automatic_env_var_set_manually),
        help(
            r#"The environment variable '{envvar_name}' is set automatically by Nushell and cannot not be set manually."#
        )
    )]
    AutomaticEnvVarSetManually {
        envvar_name: String,
        #[label("cannot set '{envvar_name}' manually")]
        span: Span,
    },

    /// It is not possible to replace the entire environment at once
    ///
    /// ## Resolution
    ///
    /// Setting the entire environment is not allowed. Change environment variables individually
    /// instead.
    #[error("Cannot replace environment.")]
    #[diagnostic(
        code(nu::shell::cannot_replace_env),
        help(r#"Assigning a value to '$env' is not allowed."#)
    )]
    CannotReplaceEnv {
        #[label("setting '$env' not allowed")]
        span: Span,
    },

    /// Division by zero is not a thing.
    ///
    /// ## Resolution
    ///
    /// Add a guard of some sort to check whether a denominator input to this division is zero, and branch off if that's the case.
    #[error("Division by zero.")]
    #[diagnostic(code(nu::shell::division_by_zero))]
    DivisionByZero {
        #[label("division by zero")]
        span: Span,
    },

    /// An error happened while tryin to create a range.
    ///
    /// This can happen in various unexpected situations, for example if the range would loop forever (as would be the case with a 0-increment).
    ///
    /// ## Resolution
    ///
    /// Check your range values to make sure they're countable and would not loop forever.
    #[error("Can't convert range to countable values")]
    #[diagnostic(code(nu::shell::range_to_countable))]
    CannotCreateRange {
        #[label = "can't convert to countable values"]
        span: Span,
    },

    /// You attempted to access an index beyond the available length of a value.
    ///
    /// ## Resolution
    ///
    /// Check your lengths and try again.
    #[error("Row number too large (max: {max_idx}).")]
    #[diagnostic(code(nu::shell::access_beyond_end))]
    AccessBeyondEnd {
        max_idx: usize,
        #[label = "index too large (max: {max_idx})"]
        span: Span,
    },

    /// You attempted to insert data at a list position higher than the end.
    ///
    /// ## Resolution
    ///
    /// To insert data into a list, assign to the last used index + 1.
    #[error("Inserted at wrong row number (should be {available_idx}).")]
    #[diagnostic(code(nu::shell::access_beyond_end))]
    InsertAfterNextFreeIndex {
        available_idx: usize,
        #[label = "can't insert at index (the next available index is {available_idx})"]
        span: Span,
    },

    /// You attempted to access an index when it's empty.
    ///
    /// ## Resolution
    ///
    /// Check your lengths and try again.
    #[error("Row number too large (empty content).")]
    #[diagnostic(code(nu::shell::access_beyond_end))]
    AccessEmptyContent {
        #[label = "index too large (empty content)"]
        span: Span,
    },

    // TODO: check to be taken over by `AccessBeyondEnd`
    /// You attempted to access an index beyond the available length of a stream.
    ///
    /// ## Resolution
    ///
    /// Check your lengths and try again.
    #[error("Row number too large.")]
    #[diagnostic(code(nu::shell::access_beyond_end_of_stream))]
    AccessBeyondEndOfStream {
        #[label = "index too large"]
        span: Span,
    },

    /// Tried to index into a type that does not support pathed access.
    ///
    /// ## Resolution
    ///
    /// Check your types. Only composite types can be pathed into.
    #[error("Data cannot be accessed with a cell path")]
    #[diagnostic(code(nu::shell::incompatible_path_access))]
    IncompatiblePathAccess {
        type_name: String,
        #[label("{type_name} doesn't support cell paths")]
        span: Span,
    },

    /// The requested column does not exist.
    ///
    /// ## Resolution
    ///
    /// Check the spelling of your column name. Did you forget to rename a column somewhere?
    #[error("Cannot find column")]
    #[diagnostic(code(nu::shell::column_not_found))]
    CantFindColumn {
        col_name: String,
        #[label = "cannot find column '{col_name}'"]
        span: Span,
        #[label = "value originates here"]
        src_span: Span,
    },

    /// Attempted to insert a column into a table, but a column with that name already exists.
    ///
    /// ## Resolution
    ///
    /// Drop or rename the existing column (check `rename -h`) and try again.
    #[error("Column already exists")]
    #[diagnostic(code(nu::shell::column_already_exists))]
    ColumnAlreadyExists {
        col_name: String,
        #[label = "column '{col_name}' already exists"]
        span: Span,
        #[label = "value originates here"]
        src_span: Span,
    },

    /// The given operation can only be performed on lists.
    ///
    /// ## Resolution
    ///
    /// Check the input type to this command. Are you sure it's a list?
    #[error("Not a list value")]
    #[diagnostic(code(nu::shell::not_a_list))]
    NotAList {
        #[label = "value not a list"]
        dst_span: Span,
        #[label = "value originates here"]
        src_span: Span,
    },

    /// Fields can only be defined once
    ///
    /// ## Resolution
    ///
    /// Check the record to ensure you aren't reusing the same field name
    #[error("Record field used twice")]
    #[diagnostic(code(nu::shell::not_a_list))]
    ColumnDefinedTwice {
        #[label = "field redefined here"]
        second_use: Span,
        #[label = "field first defined here"]
        first_use: Span,
    },

    /// An error happened while performing an external command.
    ///
    /// ## Resolution
    ///
    /// This error is fairly generic. Refer to the specific error message for further details.
    #[error("External command failed")]
    #[diagnostic(code(nu::shell::external_command), help("{help}"))]
    ExternalCommand {
        label: String,
        help: String,
        #[label("{label}")]
        span: Span,
    },

    /// An operation was attempted with an input unsupported for some reason.
    ///
    /// ## Resolution
    ///
    /// This error is fairly generic. Refer to the specific error message for further details.
    #[error("Unsupported input")]
    #[diagnostic(code(nu::shell::unsupported_input))]
    UnsupportedInput(
        String,
        String,
        #[label("{0}")] Span, // call head (the name of the command itself)
        #[label("input type: {1}")] Span,
    ),

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
    #[error("Unable to parse datetime: [{0}].")]
    #[diagnostic(
        code(nu::shell::datetime_parse_error),
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
    DatetimeParseError(String, #[label("datetime parsing failed")] Span),

    /// A network operation failed.
    ///
    /// ## Resolution
    ///
    /// It's always DNS.
    #[error("Network failure")]
    #[diagnostic(code(nu::shell::network_failure))]
    NetworkFailure(String, #[label("{0}")] Span),

    /// Help text for this command could not be found.
    ///
    /// ## Resolution
    ///
    /// Check the spelling for the requested command and try again. Are you sure it's defined and your configurations are loading correctly? Can you execute it?
    #[error("Command not found")]
    #[diagnostic(code(nu::shell::command_not_found))]
    CommandNotFound(#[label("command not found")] Span),

    /// This alias could not be found
    ///
    /// ## Resolution
    ///
    /// The alias does not exist in the current scope. It might exist in another scope or overlay or be hidden.
    #[error("Alias not found")]
    #[diagnostic(code(nu::shell::alias_not_found))]
    AliasNotFound(#[label("alias not found")] Span),

    /// A flag was not found.
    #[error("Flag not found")]
    #[diagnostic(code(nu::shell::flag_not_found))]
    // NOTE: Seems to be unused. Removable?
    FlagNotFound(String, #[label("{0} not found")] Span),

    /// Failed to find a file during a nushell operation.
    ///
    /// ## Resolution
    ///
    /// Does the file in the error message exist? Is it readable and accessible? Is the casing right?
    #[error("File not found")]
    #[diagnostic(code(nu::shell::file_not_found))]
    FileNotFound(#[label("file not found")] Span),

    /// Failed to find a file during a nushell operation.
    ///
    /// ## Resolution
    ///
    /// Does the file in the error message exist? Is it readable and accessible? Is the casing right?
    #[error("File not found")]
    #[diagnostic(code(nu::shell::file_not_found))]
    FileNotFoundCustom(String, #[label("{0}")] Span),

    /// A plugin failed to load.
    ///
    /// ## Resolution
    ///
    /// This is a fairly generic error. Refer to the specific error message for further details.
    #[error("Plugin failed to load: {0}")]
    #[diagnostic(code(nu::shell::plugin_failed_to_load))]
    PluginFailedToLoad(String),

    /// A message from a plugin failed to encode.
    ///
    /// ## Resolution
    ///
    /// This is likely a bug with the plugin itself.
    #[error("Plugin failed to encode: {0}")]
    #[diagnostic(code(nu::shell::plugin_failed_to_encode))]
    PluginFailedToEncode(String),

    /// A message to a plugin failed to decode.
    ///
    /// ## Resolution
    ///
    /// This is either an issue with the inputs to a plugin (bad JSON?) or a bug in the plugin itself. Fix or report as appropriate.
    #[error("Plugin failed to decode: {0}")]
    #[diagnostic(code(nu::shell::plugin_failed_to_decode))]
    PluginFailedToDecode(String),

    /// I/O operation interrupted.
    ///
    /// ## Resolution
    ///
    /// This is a generic error. Refer to the specific error message for further details.
    #[error("I/O interrupted")]
    #[diagnostic(code(nu::shell::io_interrupted))]
    IOInterrupted(String, #[label("{0}")] Span),

    /// An I/O operation failed.
    ///
    /// ## Resolution
    ///
    /// This is a generic error. Refer to the specific error message for further details.
    #[error("I/O error")]
    #[diagnostic(code(nu::shell::io_error), help("{0}"))]
    IOError(String),

    /// An I/O operation failed.
    ///
    /// ## Resolution
    ///
    /// This is a generic error. Refer to the specific error message for further details.
    #[error("I/O error")]
    #[diagnostic(code(nu::shell::io_error))]
    IOErrorSpanned(String, #[label("{0}")] Span),

    /// Permission for an operation was denied.
    ///
    /// ## Resolution
    ///
    /// This is a generic error. Refer to the specific error message for further details.
    #[error("Permission Denied")]
    #[diagnostic(code(nu::shell::permission_denied))]
    PermissionDeniedError(String, #[label("{0}")] Span),

    /// Out of memory.
    ///
    /// ## Resolution
    ///
    /// This is a generic error. Refer to the specific error message for further details.
    #[error("Out of memory")]
    #[diagnostic(code(nu::shell::out_of_memory))]
    OutOfMemoryError(String, #[label("{0}")] Span),

    /// Tried to `cd` to a path that isn't a directory.
    ///
    /// ## Resolution
    ///
    /// Make sure the path is a directory. It currently exists, but is of some other type, like a file.
    #[error("Cannot change to directory")]
    #[diagnostic(code(nu::shell::cannot_cd_to_directory))]
    NotADirectory(#[label("is not a directory")] Span),

    /// Attempted to perform an operation on a directory that doesn't exist.
    ///
    /// ## Resolution
    ///
    /// Make sure the directory in the error message actually exists before trying again.
    #[error("Directory not found")]
    #[diagnostic(code(nu::shell::directory_not_found))]
    DirectoryNotFound(#[label("directory not found")] Span, #[help] Option<String>),

    /// Attempted to perform an operation on a directory that doesn't exist.
    ///
    /// ## Resolution
    ///
    /// Make sure the directory in the error message actually exists before trying again.
    #[error("Directory not found")]
    #[diagnostic(code(nu::shell::directory_not_found_custom))]
    DirectoryNotFoundCustom(String, #[label("{0}")] Span),

    /// The requested move operation cannot be completed. This is typically because both paths exist,
    /// but are of different types. For example, you might be trying to overwrite an existing file with
    /// a directory.
    ///
    /// ## Resolution
    ///
    /// Make sure the destination path does not exist before moving a directory.
    #[error("Move not possible")]
    #[diagnostic(code(nu::shell::move_not_possible))]
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
    #[diagnostic(code(nu::shell::move_not_possible_single))]
    // NOTE: Currently not actively used.
    MoveNotPossibleSingle(String, #[label("{0}")] Span),

    /// Failed to create either a file or directory.
    ///
    /// ## Resolution
    ///
    /// This is a fairly generic error. Refer to the specific error message for further details.
    #[error("Create not possible")]
    #[diagnostic(code(nu::shell::create_not_possible))]
    CreateNotPossible(String, #[label("{0}")] Span),

    /// Changing the access time ("atime") of this file is not possible.
    ///
    /// ## Resolution
    ///
    /// This can be for various reasons, such as your platform or permission flags. Refer to the specific error message for more details.
    #[error("Not possible to change the access time")]
    #[diagnostic(code(nu::shell::change_access_time_not_possible))]
    ChangeAccessTimeNotPossible(String, #[label("{0}")] Span),

    /// Changing the modification time ("mtime") of this file is not possible.
    ///
    /// ## Resolution
    ///
    /// This can be for various reasons, such as your platform or permission flags. Refer to the specific error message for more details.
    #[error("Not possible to change the modified time")]
    #[diagnostic(code(nu::shell::change_modified_time_not_possible))]
    ChangeModifiedTimeNotPossible(String, #[label("{0}")] Span),

    /// Unable to remove this item.
    #[error("Remove not possible")]
    #[diagnostic(code(nu::shell::remove_not_possible))]
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
    #[diagnostic(code(nu::shell::error_reading_file))]
    ReadingFile(String, #[label("{0}")] Span),

    /// A name was not found. Did you mean a different name?
    ///
    /// ## Resolution
    ///
    /// The error message will suggest a possible match for what you meant.
    #[error("Name not found")]
    #[diagnostic(code(nu::shell::name_not_found))]
    DidYouMean(String, #[label("did you mean '{0}'?")] Span),

    /// A name was not found. Did you mean a different name?
    ///
    /// ## Resolution
    ///
    /// The error message will suggest a possible match for what you meant.
    #[error("{0}")]
    #[diagnostic(code(nu::shell::did_you_mean_custom))]
    DidYouMeanCustom(String, String, #[label("did you mean '{1}'?")] Span),

    /// The given input must be valid UTF-8 for further processing.
    ///
    /// ## Resolution
    ///
    /// Check your input's encoding. Are there any funny characters/bytes?
    #[error("Non-UTF8 string")]
    #[diagnostic(code(nu::parser::non_utf8))]
    NonUtf8(#[label = "non-UTF8 string"] Span),

    /// The given input must be valid UTF-8 for further processing.
    ///
    /// ## Resolution
    ///
    /// Check your input's encoding. Are there any funny characters/bytes?
    #[error("Non-UTF8 string")]
    #[diagnostic(code(nu::parser::non_utf8_custom))]
    NonUtf8Custom(String, #[label = "{0}"] Span),

    /// A custom value could not be converted to a Dataframe.
    ///
    /// ## Resolution
    ///
    /// Make sure conversion to a Dataframe is possible for this value or convert it to a type that does, first.
    #[error("Casting error")]
    #[diagnostic(code(nu::shell::downcast_not_possible))]
    DowncastNotPossible(String, #[label("{0}")] Span),

    /// The value given for this configuration is not supported.
    ///
    /// ## Resolution
    ///
    /// Refer to the specific error message for details and convert values as needed.
    #[error("Unsupported config value")]
    #[diagnostic(code(nu::shell::unsupported_config_value))]
    UnsupportedConfigValue(String, String, #[label = "expected {0}, got {1}"] Span),

    /// An expected configuration value is not present.
    ///
    /// ## Resolution
    ///
    /// Refer to the specific error message and add the configuration value to your config file as needed.
    #[error("Missing config value")]
    #[diagnostic(code(nu::shell::missing_config_value))]
    MissingConfigValue(String, #[label = "missing {0}"] Span),

    /// Negative value passed when positive one is required.
    ///
    /// ## Resolution
    ///
    /// Guard against negative values or check your inputs.
    #[error("Negative value passed when positive one is required")]
    #[diagnostic(code(nu::shell::needs_positive_value))]
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
    #[diagnostic(code(nu::shell::deprecated_command))]
    DeprecatedCommand(
        String,
        String,
        #[label = "'{0}' is deprecated. Please use '{1}' instead."] Span,
    ),

    /// Attempted to use a deprecated parameter.
    ///
    /// ## Resolution
    ///
    /// Check the help for the command and update your script accordingly.
    #[error("Deprecated parameter {0}")]
    #[diagnostic(code(nu::shell::deprecated_command))]
    DeprecatedParameter(
        String,
        String,
        #[label = "Parameter '{0}' is deprecated. Please use '{1}' instead."] Span,
    ),

    /// Non-Unicode input received.
    ///
    /// ## Resolution
    ///
    /// Check that your path is UTF-8 compatible.
    #[error("Non-Unicode input received.")]
    #[diagnostic(code(nu::shell::non_unicode_input))]
    NonUnicodeInput,

    /// Unexpected abbr component.
    ///
    /// ## Resolution
    ///
    /// Check the path abbreviation to ensure that it is valid.
    #[error("Unexpected abbr component `{0}`.")]
    #[diagnostic(code(nu::shell::unexpected_path_abbreviateion))]
    UnexpectedAbbrComponent(String),

    // It should be only used by commands accepts block, and accept inputs from pipeline.
    /// Failed to eval block with specific pipeline input.
    #[error("Eval block failed with pipeline input")]
    #[diagnostic(code(nu::shell::eval_block_with_input))]
    EvalBlockWithInput(#[label("source value")] Span, #[related] Vec<ShellError>),

    /// Break event, which may become an error if used outside of a loop
    #[error("Break used outside of loop")]
    Break(#[label = "used outside of loop"] Span),

    /// Continue event, which may become an error if used outside of a loop
    #[error("Continue used outside of loop")]
    Continue(#[label = "used outside of loop"] Span),

    /// Return event, which may become an error if used outside of a function
    #[error("Return used outside of function")]
    Return(#[label = "used outside of function"] Span, Box<Value>),

    /// The code being executed called itself too many times.
    ///
    /// ## Resolution
    ///
    /// Adjust your Nu code to
    #[error("Recursion limit ({recursion_limit}) reached")]
    #[diagnostic(code(nu::shell::recursion_limit_reached))]
    RecursionLimitReached {
        recursion_limit: u64,
        #[label("This called itself too many times")]
        span: Option<Span>,
    },

    /// An attempt to access a record column failed.
    #[error("Access failure: {message}")]
    #[diagnostic(code(nu::shell::lazy_record_access_failed))]
    LazyRecordAccessFailed {
        message: String,
        column_name: String,
        #[label("Could not access '{column_name}' on this record")]
        span: Span,
    },

    /// Operation interrupted by user
    #[error("Operation interrupted by user")]
    InterruptedByUser {
        #[label("This operation was interrupted")]
        span: Option<Span>,
    },
}

impl From<std::io::Error> for ShellError {
    fn from(input: std::io::Error) -> ShellError {
        ShellError::IOError(format!("{input:?}"))
    }
}

impl std::convert::From<Box<dyn std::error::Error>> for ShellError {
    fn from(input: Box<dyn std::error::Error>) -> ShellError {
        ShellError::IOError(input.to_string())
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for ShellError {
    fn from(input: Box<dyn std::error::Error + Send + Sync>) -> ShellError {
        ShellError::IOError(format!("{input:?}"))
    }
}

pub fn into_code(err: &ShellError) -> Option<String> {
    err.code().map(|code| code.to_string())
}
