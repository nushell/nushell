use crate::{Span, ast::Call};

#[derive(Debug, Clone)]
pub struct UnevaluatedCallInfo {
    pub args: Call,
    pub name_span: Span,
}
