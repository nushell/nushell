use crate::{Span, Type};

#[derive(Debug)]
pub enum ShellError {
    OperatorMismatch {
        op_span: Span,
        lhs_ty: Type,
        lhs_span: Span,
        rhs_ty: Type,
        rhs_span: Span,
    },
    Unsupported(Span),
    InternalError(String),
    VariableNotFound(Span),
    CantConvert(String, Span),
    DivisionByZero(Span),
}
