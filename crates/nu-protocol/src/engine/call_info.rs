use crate::{ast::Call, FutureSpanId};

#[derive(Debug, Clone)]
pub struct UnevaluatedCallInfo {
    pub args: Call,
    pub name_span: FutureSpanId,
}
