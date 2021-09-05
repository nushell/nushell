use crate::{ast::Operator, Span, Type};

#[derive(Debug, Clone)]
pub enum ShellError {
    OperatorMismatch {
        op_span: Span,
        lhs_ty: Type,
        lhs_span: Span,
        rhs_ty: Type,
        rhs_span: Span,
    },
    UnsupportedOperator(Operator, Span),
    UnknownOperator(String, Span),
    ExternalNotSupported(Span),
    InternalError(String),
    VariableNotFound(Span),
    CantConvert(String, Span),
    DivisionByZero(Span),
}
