use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    ast::Operator, engine::StateWorkingSet, format_error, LabeledError, ParseError, Span, Spanned,
    Value,
};

/// The fundamental error type for the evaluation engine. These cases represent different kinds of errors
/// the evaluator might face, along with helpful spans to label. An error renderer will take this error value
/// and pass it into an error viewer to display to the user.
#[derive(Debug, Clone, Error, Diagnostic, PartialEq)]
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
        lhs_ty: String,
        #[label("{lhs_ty}")]
        lhs_span: Span,
        rhs_ty: String,
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
        val_span: Span,
        #[label = "encountered here"]
        call_span: Span,
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
    /// It is very likely that this is a bug. Please file an issue at <https://github.com/nushell/nushell/issues> with relevant information.
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
    /// It is very likely that this is a bug. Please file an issue at <https://github.com/nushell/nushell/issues> with relevant information.
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
    /// It is very likely that this is a bug. Please file an issue at <https://github.com/nushell/nushell/issues> with relevant information.
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

    #[error("Can't convert string `{details}` to duration.")]
    #[diagnostic(code(nu::shell::cant_convert_with_value))]
    CantConvertToDuration {
        details: String,
        #[label("can't be converted to duration")]
        dst_span: Span,
        #[label("this string value...")]
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
            r#"The environment variable '{envvar_name}' is set automatically by Nushell and cannot be set manually."#
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

    /// An error happened while trying to create a range.
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
    #[error("Record field or table column used twice: {col_name}")]
    #[diagnostic(code(nu::shell::column_defined_twice))]
    ColumnDefinedTwice {
        col_name: String,
        #[label = "field redefined here"]
        second_use: Span,
        #[label = "field first defined here"]
        first_use: Span,
    },

    /// Attempted to create a record from different number of columns and values
    ///
    /// ## Resolution
    ///
    /// Check the record has the same number of columns as values
    #[error("Attempted to create a record from different number of columns and values")]
    #[diagnostic(code(nu::shell::record_cols_vals_mismatch))]
    RecordColsValsMismatch {
        #[label = "problematic value"]
        bad_value: Span,
        #[label = "attempted to create the record here"]
        creation_site: Span,
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
    UnsupportedInput {
        msg: String,
        input: String,
        #[label("{msg}")]
        msg_span: Span,
        #[label("{input}")]
        input_span: Span,
    },

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
    #[error("Unable to parse datetime: [{msg}].")]
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
    DatetimeParseError {
        msg: String,
        #[label("datetime parsing failed")]
        span: Span,
    },

    /// A network operation failed.
    ///
    /// ## Resolution
    ///
    /// It's always DNS.
    #[error("Network failure")]
    #[diagnostic(code(nu::shell::network_failure))]
    NetworkFailure {
        msg: String,
        #[label("{msg}")]
        span: Span,
    },

    /// Help text for this command could not be found.
    ///
    /// ## Resolution
    ///
    /// Check the spelling for the requested command and try again. Are you sure it's defined and your configurations are loading correctly? Can you execute it?
    #[error("Command not found")]
    #[diagnostic(code(nu::shell::command_not_found))]
    CommandNotFound {
        #[label("command not found")]
        span: Span,
    },

    /// This alias could not be found
    ///
    /// ## Resolution
    ///
    /// The alias does not exist in the current scope. It might exist in another scope or overlay or be hidden.
    #[error("Alias not found")]
    #[diagnostic(code(nu::shell::alias_not_found))]
    AliasNotFound {
        #[label("alias not found")]
        span: Span,
    },

    /// Failed to find a file during a nushell operation.
    ///
    /// ## Resolution
    ///
    /// Does the file in the error message exist? Is it readable and accessible? Is the casing right?
    #[error("File not found")]
    #[diagnostic(code(nu::shell::file_not_found), help("{file} does not exist"))]
    FileNotFound {
        file: String,
        #[label("file not found")]
        span: Span,
    },

    /// Failed to find a file during a nushell operation.
    ///
    /// ## Resolution
    ///
    /// Does the file in the error message exist? Is it readable and accessible? Is the casing right?
    #[error("File not found")]
    #[diagnostic(code(nu::shell::file_not_found))]
    FileNotFoundCustom {
        msg: String,
        #[label("{msg}")]
        span: Span,
    },

    /// The cached plugin data (in `$nu.plugin-path`) for a plugin is invalid.
    ///
    /// ## Resolution
    ///
    /// `register` the plugin again to update the data, or remove it.
    #[error("The cached plugin data for `{plugin_name}` is invalid")]
    #[diagnostic(code(nu::shell::plugin_cache_data_invalid))]
    PluginCacheDataInvalid {
        plugin_name: String,
        #[help("try registering the plugin again with `{}`")]
        register_command: String,
    },

    /// A plugin failed to load.
    ///
    /// ## Resolution
    ///
    /// This is a fairly generic error. Refer to the specific error message for further details.
    #[error("Plugin failed to load: {msg}")]
    #[diagnostic(code(nu::shell::plugin_failed_to_load))]
    PluginFailedToLoad { msg: String },

    /// A message from a plugin failed to encode.
    ///
    /// ## Resolution
    ///
    /// This is likely a bug with the plugin itself.
    #[error("Plugin failed to encode: {msg}")]
    #[diagnostic(code(nu::shell::plugin_failed_to_encode))]
    PluginFailedToEncode { msg: String },

    /// A message to a plugin failed to decode.
    ///
    /// ## Resolution
    ///
    /// This is either an issue with the inputs to a plugin (bad JSON?) or a bug in the plugin itself. Fix or report as appropriate.
    #[error("Plugin failed to decode: {msg}")]
    #[diagnostic(code(nu::shell::plugin_failed_to_decode))]
    PluginFailedToDecode { msg: String },

    /// A custom value cannot be sent to the given plugin.
    ///
    /// ## Resolution
    ///
    /// Custom values can only be used with the plugin they came from. Use a command from that
    /// plugin instead.
    #[error("Custom value `{name}` cannot be sent to plugin")]
    #[diagnostic(code(nu::shell::custom_value_incorrect_for_plugin))]
    CustomValueIncorrectForPlugin {
        name: String,
        #[label("the `{dest_plugin}` plugin does not support this kind of value")]
        span: Span,
        dest_plugin: String,
        #[help("this value came from the `{}` plugin")]
        src_plugin: Option<String>,
    },

    /// The plugin failed to encode a custom value.
    ///
    /// ## Resolution
    ///
    /// This is likely a bug with the plugin itself. The plugin may have tried to send a custom
    /// value that is not serializable.
    #[error("Custom value failed to encode")]
    #[diagnostic(code(nu::shell::custom_value_failed_to_encode))]
    CustomValueFailedToEncode {
        msg: String,
        #[label("{msg}")]
        span: Span,
    },

    /// The plugin failed to encode a custom value.
    ///
    /// ## Resolution
    ///
    /// This may be a bug within the plugin, or the plugin may have been updated in between the
    /// creation of the custom value and its use.
    #[error("Custom value failed to decode")]
    #[diagnostic(code(nu::shell::custom_value_failed_to_decode))]
    #[diagnostic(help(
        "the plugin may have been updated and no longer support this custom value"
    ))]
    CustomValueFailedToDecode {
        msg: String,
        #[label("{msg}")]
        span: Span,
    },

    /// I/O operation interrupted.
    ///
    /// ## Resolution
    ///
    /// This is a generic error. Refer to the specific error message for further details.
    #[error("I/O interrupted")]
    #[diagnostic(code(nu::shell::io_interrupted))]
    IOInterrupted {
        msg: String,
        #[label("{msg}")]
        span: Span,
    },

    /// An I/O operation failed.
    ///
    /// ## Resolution
    ///
    /// This is a generic error. Refer to the specific error message for further details.
    #[error("I/O error")]
    #[diagnostic(code(nu::shell::io_error), help("{msg}"))]
    IOError { msg: String },

    /// An I/O operation failed.
    ///
    /// ## Resolution
    ///
    /// This is a generic error. Refer to the specific error message for further details.
    #[error("I/O error")]
    #[diagnostic(code(nu::shell::io_error))]
    IOErrorSpanned {
        msg: String,
        #[label("{msg}")]
        span: Span,
    },

    /// Tried to `cd` to a path that isn't a directory.
    ///
    /// ## Resolution
    ///
    /// Make sure the path is a directory. It currently exists, but is of some other type, like a file.
    #[error("Cannot change to directory")]
    #[diagnostic(code(nu::shell::cannot_cd_to_directory))]
    NotADirectory {
        #[label("is not a directory")]
        span: Span,
    },

    /// Attempted to perform an operation on a directory that doesn't exist.
    ///
    /// ## Resolution
    ///
    /// Make sure the directory in the error message actually exists before trying again.
    #[error("Directory not found")]
    #[diagnostic(code(nu::shell::directory_not_found), help("{dir} does not exist"))]
    DirectoryNotFound {
        dir: String,
        #[label("directory not found")]
        span: Span,
    },

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

    /// Failed to create either a file or directory.
    ///
    /// ## Resolution
    ///
    /// This is a fairly generic error. Refer to the specific error message for further details.
    #[error("Create not possible")]
    #[diagnostic(code(nu::shell::create_not_possible))]
    CreateNotPossible {
        msg: String,
        #[label("{msg}")]
        span: Span,
    },

    /// Changing the access time ("atime") of this file is not possible.
    ///
    /// ## Resolution
    ///
    /// This can be for various reasons, such as your platform or permission flags. Refer to the specific error message for more details.
    #[error("Not possible to change the access time")]
    #[diagnostic(code(nu::shell::change_access_time_not_possible))]
    ChangeAccessTimeNotPossible {
        msg: String,
        #[label("{msg}")]
        span: Span,
    },

    /// Changing the modification time ("mtime") of this file is not possible.
    ///
    /// ## Resolution
    ///
    /// This can be for various reasons, such as your platform or permission flags. Refer to the specific error message for more details.
    #[error("Not possible to change the modified time")]
    #[diagnostic(code(nu::shell::change_modified_time_not_possible))]
    ChangeModifiedTimeNotPossible {
        msg: String,
        #[label("{msg}")]
        span: Span,
    },

    /// Unable to remove this item.
    ///
    /// ## Resolution
    ///
    /// Removal can fail for a number of reasons, such as permissions problems. Refer to the specific error message for more details.
    #[error("Remove not possible")]
    #[diagnostic(code(nu::shell::remove_not_possible))]
    RemoveNotPossible {
        msg: String,
        #[label("{msg}")]
        span: Span,
    },

    /// Error while trying to read a file
    ///
    /// ## Resolution
    ///
    /// The error will show the result from a file operation
    #[error("Error trying to read file")]
    #[diagnostic(code(nu::shell::error_reading_file))]
    ReadingFile {
        msg: String,
        #[label("{msg}")]
        span: Span,
    },

    /// A name was not found. Did you mean a different name?
    ///
    /// ## Resolution
    ///
    /// The error message will suggest a possible match for what you meant.
    #[error("Name not found")]
    #[diagnostic(code(nu::shell::name_not_found))]
    DidYouMean {
        suggestion: String,
        #[label("did you mean '{suggestion}'?")]
        span: Span,
    },

    /// A name was not found. Did you mean a different name?
    ///
    /// ## Resolution
    ///
    /// The error message will suggest a possible match for what you meant.
    #[error("{msg}")]
    #[diagnostic(code(nu::shell::did_you_mean_custom))]
    DidYouMeanCustom {
        msg: String,
        suggestion: String,
        #[label("did you mean '{suggestion}'?")]
        span: Span,
    },

    /// The given input must be valid UTF-8 for further processing.
    ///
    /// ## Resolution
    ///
    /// Check your input's encoding. Are there any funny characters/bytes?
    #[error("Non-UTF8 string")]
    #[diagnostic(code(nu::parser::non_utf8))]
    NonUtf8 {
        #[label("non-UTF8 string")]
        span: Span,
    },

    /// The given input must be valid UTF-8 for further processing.
    ///
    /// ## Resolution
    ///
    /// Check your input's encoding. Are there any funny characters/bytes?
    #[error("Non-UTF8 string")]
    #[diagnostic(code(nu::parser::non_utf8_custom))]
    NonUtf8Custom {
        msg: String,
        #[label("{msg}")]
        span: Span,
    },

    /// A custom value could not be converted to a Dataframe.
    ///
    /// ## Resolution
    ///
    /// Make sure conversion to a Dataframe is possible for this value or convert it to a type that does, first.
    #[error("Casting error")]
    #[diagnostic(code(nu::shell::downcast_not_possible))]
    DowncastNotPossible {
        msg: String,
        #[label("{msg}")]
        span: Span,
    },

    /// The value given for this configuration is not supported.
    ///
    /// ## Resolution
    ///
    /// Refer to the specific error message for details and convert values as needed.
    #[error("Unsupported config value")]
    #[diagnostic(code(nu::shell::unsupported_config_value))]
    UnsupportedConfigValue {
        expected: String,
        value: String,
        #[label("expected {expected}, got {value}")]
        span: Span,
    },

    /// An expected configuration value is not present.
    ///
    /// ## Resolution
    ///
    /// Refer to the specific error message and add the configuration value to your config file as needed.
    #[error("Missing config value")]
    #[diagnostic(code(nu::shell::missing_config_value))]
    MissingConfigValue {
        missing_value: String,
        #[label("missing {missing_value}")]
        span: Span,
    },

    /// Negative value passed when positive one is required.
    ///
    /// ## Resolution
    ///
    /// Guard against negative values or check your inputs.
    #[error("Negative value passed when positive one is required")]
    #[diagnostic(code(nu::shell::needs_positive_value))]
    NeedsPositiveValue {
        #[label("use a positive value")]
        span: Span,
    },

    /// This is a generic error type used for different situations.
    #[error("{error}")]
    #[diagnostic()]
    GenericError {
        error: String,
        msg: String,
        #[label("{msg}")]
        span: Option<Span>,
        #[help]
        help: Option<String>,
        #[related]
        inner: Vec<ShellError>,
    },

    /// This is a generic error type used for different situations.
    #[error("{error}")]
    #[diagnostic()]
    OutsideSpannedLabeledError {
        #[source_code]
        src: String,
        error: String,
        msg: String,
        #[label("{msg}")]
        span: Span,
    },

    /// This is a generic error type used for user and plugin-generated errors.
    #[error(transparent)]
    #[diagnostic(transparent)]
    LabeledError(#[from] Box<super::LabeledError>),

    /// Attempted to use a command that has been removed from Nushell.
    ///
    /// ## Resolution
    ///
    /// Check the help for the new suggested command and update your script accordingly.
    #[error("Removed command: {removed}")]
    #[diagnostic(code(nu::shell::removed_command))]
    RemovedCommand {
        removed: String,
        replacement: String,
        #[label("'{removed}' has been removed from Nushell. Please use '{replacement}' instead.")]
        span: Span,
    },

    // It should be only used by commands accepts block, and accept inputs from pipeline.
    /// Failed to eval block with specific pipeline input.
    #[error("Eval block failed with pipeline input")]
    #[diagnostic(code(nu::shell::eval_block_with_input))]
    EvalBlockWithInput {
        #[label("source value")]
        span: Span,
        #[related]
        sources: Vec<ShellError>,
    },

    /// Break event, which may become an error if used outside of a loop
    #[error("Break used outside of loop")]
    Break {
        #[label("used outside of loop")]
        span: Span,
    },

    /// Continue event, which may become an error if used outside of a loop
    #[error("Continue used outside of loop")]
    Continue {
        #[label("used outside of loop")]
        span: Span,
    },

    /// Return event, which may become an error if used outside of a function
    #[error("Return used outside of function")]
    Return {
        #[label("used outside of function")]
        span: Span,
        value: Box<Value>,
    },

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

    /// An attempt to use, as a match guard, an expression that
    /// does not resolve into a boolean
    #[error("Match guard not bool")]
    #[diagnostic(
        code(nu::shell::match_guard_not_bool),
        help("Match guards should evaluate to a boolean")
    )]
    MatchGuardNotBool {
        #[label("not a boolean expression")]
        span: Span,
    },

    /// An attempt to run a command marked for constant evaluation lacking the const. eval.
    /// implementation.
    ///
    /// This is an internal Nushell error, please file an issue.
    #[error("Missing const eval implementation")]
    #[diagnostic(
        code(nu::shell::missing_const_eval_implementation),
        help(
            "The command lacks an implementation for constant evaluation. \
This is an internal Nushell error, please file an issue https://github.com/nushell/nushell/issues."
        )
    )]
    MissingConstEvalImpl {
        #[label("command lacks constant implementation")]
        span: Span,
    },

    /// Tried assigning non-constant value to a constant
    ///
    /// ## Resolution
    ///
    /// Only a subset of expressions are allowed to be assigned as a constant during parsing.
    #[error("Not a constant.")]
    #[diagnostic(
        code(nu::shell::not_a_constant),
        help("Only a subset of expressions are allowed constants during parsing. Try using the 'const' command or typing the value literally.")
    )]
    NotAConstant {
        #[label("Value is not a parse-time constant")]
        span: Span,
    },

    /// Tried running a command that is not const-compatible
    ///
    /// ## Resolution
    ///
    /// Only a subset of builtin commands, and custom commands built only from those commands, can
    /// run at parse time.
    #[error("Not a const command.")]
    #[diagnostic(
        code(nu::shell::not_a_const_command),
        help("Only a subset of builtin commands, and custom commands built only from those commands, can run at parse time.")
    )]
    NotAConstCommand {
        #[label("This command cannot run at parse time.")]
        span: Span,
    },

    /// Tried getting a help message at parse time.
    ///
    /// ## Resolution
    ///
    /// Help messages are not supported at parse time.
    #[error("Help message not a constant.")]
    #[diagnostic(
        code(nu::shell::not_a_const_help),
        help("Help messages are currently not supported to be constants.")
    )]
    NotAConstHelp {
        #[label("This command cannot run at parse time.")]
        span: Span,
    },

    /// Invalid glob pattern
    ///
    /// ## Resolution
    ///
    /// Correct glob pattern
    #[error("Invalid glob pattern")]
    #[diagnostic(
        code(nu::shell::invalid_glob_pattern),
        help("Refer to xxx for help on nushell glob patterns.")
    )]
    InvalidGlobPattern {
        msg: String,
        #[label("{msg}")]
        span: Span,
    },

    /// Tried spreading a non-list inside a list or command call.
    ///
    /// ## Resolution
    ///
    /// Only lists can be spread inside lists and command calls. Try converting the value to a list before spreading.
    #[error("Not a list")]
    #[diagnostic(
        code(nu::shell::cannot_spread_as_list),
        help("Only lists can be spread inside lists and command calls. Try converting the value to a list before spreading.")
    )]
    CannotSpreadAsList {
        #[label = "cannot spread value"]
        span: Span,
    },

    /// Tried spreading a non-record inside a record.
    ///
    /// ## Resolution
    ///
    /// Only records can be spread inside records. Try converting the value to a record before spreading.
    #[error("Not a record")]
    #[diagnostic(
        code(nu::shell::cannot_spread_as_record),
        help("Only records can be spread inside records. Try converting the value to a record before spreading.")
    )]
    CannotSpreadAsRecord {
        #[label = "cannot spread value"]
        span: Span,
    },

    /// Lists are not automatically spread when calling external commands
    ///
    /// ## Resolution
    ///
    /// Use the spread operator (put a '...' before the argument)
    #[error("Lists are not automatically spread when calling external commands")]
    #[diagnostic(
        code(nu::shell::cannot_pass_list_to_external),
        help("Either convert the list to a string or use the spread operator, like so: ...{arg}")
    )]
    CannotPassListToExternal {
        arg: String,
        #[label = "Spread operator (...) is necessary to spread lists"]
        span: Span,
    },

    /// Out of bounds.
    ///
    /// ## Resolution
    ///
    /// Make sure the range is within the bounds of the input.
    #[error(
        "The selected range {left_flank}..{right_flank} is out of the bounds of the provided input"
    )]
    #[diagnostic(code(nu::shell::out_of_bounds))]
    OutOfBounds {
        left_flank: String,
        right_flank: String,
        #[label = "byte index is not a char boundary or is out of bounds of the input"]
        span: Span,
    },

    /// The config directory could not be found
    #[error("The config directory could not be found")]
    #[diagnostic(
        code(nu::shell::config_dir_not_found),
        help(
            r#"On Linux, this would be $XDG_CONFIG_HOME or $HOME/.config.
On MacOS, this would be `$HOME/Library/Application Support`.
On Windows, this would be %USERPROFILE%\AppData\Roaming"#
        )
    )]
    ConfigDirNotFound {
        #[label = "Could not find config directory"]
        span: Option<Span>,
    },

    /// XDG_CONFIG_HOME was set to an invalid path
    #[error("$env.XDG_CONFIG_HOME ({xdg}) is invalid, using default config directory instead: {default}")]
    #[diagnostic(
        code(nu::shell::xdg_config_home_invalid),
        help("Set XDG_CONFIG_HOME to an absolute path, or set it to an empty string to ignore it")
    )]
    InvalidXdgConfig { xdg: String, default: String },
}

// TODO: Implement as From trait
impl ShellError {
    pub fn wrap(self, working_set: &StateWorkingSet, span: Span) -> ParseError {
        let msg = format_error(working_set, &self);
        ParseError::LabeledError(
            msg,
            "Encountered error during parse-time evaluation".into(),
            span,
        )
    }
}

impl From<std::io::Error> for ShellError {
    fn from(input: std::io::Error) -> ShellError {
        ShellError::IOError {
            msg: format!("{input:?}"),
        }
    }
}

impl From<Spanned<std::io::Error>> for ShellError {
    fn from(error: Spanned<std::io::Error>) -> Self {
        ShellError::IOErrorSpanned {
            msg: error.item.to_string(),
            span: error.span,
        }
    }
}

impl std::convert::From<Box<dyn std::error::Error>> for ShellError {
    fn from(input: Box<dyn std::error::Error>) -> ShellError {
        ShellError::IOError {
            msg: input.to_string(),
        }
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for ShellError {
    fn from(input: Box<dyn std::error::Error + Send + Sync>) -> ShellError {
        ShellError::IOError {
            msg: format!("{input:?}"),
        }
    }
}

impl From<super::LabeledError> for ShellError {
    fn from(value: super::LabeledError) -> Self {
        ShellError::LabeledError(Box::new(value))
    }
}

/// `ShellError` always serializes as [`LabeledError`].
impl Serialize for ShellError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        LabeledError::from_diagnostic(self).serialize(serializer)
    }
}

/// `ShellError` always deserializes as if it were [`LabeledError`], resulting in a
/// [`ShellError::LabeledError`] variant.
impl<'de> Deserialize<'de> for ShellError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        LabeledError::deserialize(deserializer).map(ShellError::from)
    }
}

pub fn into_code(err: &ShellError) -> Option<String> {
    err.code().map(|code| code.to_string())
}

#[test]
fn shell_error_serialize_roundtrip() {
    // Ensure that we can serialize and deserialize `ShellError`, and check that it basically would
    // look the same
    let original_error = ShellError::CantConvert {
        span: Span::new(100, 200),
        to_type: "Foo".into(),
        from_type: "Bar".into(),
        help: Some("this is a test".into()),
    };
    println!("orig_error = {:#?}", original_error);

    let serialized =
        serde_json::to_string_pretty(&original_error).expect("serde_json::to_string_pretty failed");
    println!("serialized = {}", serialized);

    let deserialized: ShellError =
        serde_json::from_str(&serialized).expect("serde_json::from_str failed");
    println!("deserialized = {:#?}", deserialized);

    // We don't expect the deserialized error to be the same as the original error, but its miette
    // properties should be comparable
    assert_eq!(original_error.to_string(), deserialized.to_string());

    assert_eq!(
        original_error.code().map(|c| c.to_string()),
        deserialized.code().map(|c| c.to_string())
    );

    let orig_labels = original_error
        .labels()
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let deser_labels = deserialized
        .labels()
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    assert_eq!(orig_labels, deser_labels);

    assert_eq!(
        original_error.help().map(|c| c.to_string()),
        deserialized.help().map(|c| c.to_string())
    );
}
