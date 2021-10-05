use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{ast::Operator, Span, Type};

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

    #[error("Unsupported operator: {0}.")]
    #[diagnostic(code(nu::shell::unsupported_operator), url(docsrs))]
    UnsupportedOperator(Operator, #[label = "unsupported operator"] Span),

    #[error("Unsupported operator: {0}.")]
    #[diagnostic(code(nu::shell::unknown_operator), url(docsrs))]
    UnknownOperator(String, #[label = "unsupported operator"] Span),

    #[error("External commands not yet supported")]
    #[diagnostic(code(nu::shell::external_commands), url(docsrs))]
    ExternalNotSupported(#[label = "external not supported"] Span),

    #[error("Internal error: {0}.")]
    #[diagnostic(code(nu::shell::internal_error), url(docsrs))]
    InternalError(String),

    #[error("Variable not found")]
    #[diagnostic(code(nu::shell::variable_not_found), url(docsrs))]
    VariableNotFoundAtRuntime(#[label = "variable not found"] Span),

    #[error("Can't convert to {0}.")]
    #[diagnostic(code(nu::shell::cant_convert), url(docsrs))]
    CantConvert(String, #[label("can't convert to {0}")] Span),

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
    CantFindColumn(#[label = "cannot find column"] Span),

    #[error("External command")]
    #[diagnostic(code(nu::shell::external_command), url(docsrs))]
    ExternalCommand(String, #[label("{0}")] Span),

    #[error("Unsupported input")]
    #[diagnostic(code(nu::shell::unsupported_input), url(docsrs))]
    UnsupportedInput(String, #[label("{0}")] Span),

    #[error("Flag not found")]
    #[diagnostic(code(nu::shell::flag_not_found), url(docsrs))]
    FlagNotFound(String, #[label("{0} not found")] Span),

    #[error("File not found")]
    #[diagnostic(code(nu::shell::file_not_found), url(docsrs))]
    FileNotFound(#[label("file not found")] Span),

    #[error("Directory not found")]
    #[diagnostic(code(nu::shell::directory_not_found), url(docsrs))]
    DirectoryNotFound(#[label("directory not found")] Span),

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
}
