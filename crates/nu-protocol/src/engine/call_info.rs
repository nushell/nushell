use crate::ast::Call;
use crate::Span;

#[derive(Debug, Clone)]
pub struct UnevaluatedCallInfo {
    pub args: Call,
    pub name_span: Span,
}
