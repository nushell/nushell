use crate::{ast::Call, Span};

#[derive(Debug, Clone)]
pub struct UnevaluatedCallInfo {
    pub args: Call,
    pub name_span: Span,
}
