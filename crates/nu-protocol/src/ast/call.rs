use super::Expression;
use crate::{DeclId, Span};

#[derive(Debug, Clone)]
pub struct Call {
    /// identifier of the declaration to call
    pub decl_id: DeclId,
    pub head: Span,
    pub positional: Vec<Expression>,
    pub named: Vec<(String, Option<Expression>)>,
}

impl Default for Call {
    fn default() -> Self {
        Self::new()
    }
}

impl Call {
    pub fn new() -> Call {
        Self {
            decl_id: 0,
            head: Span::unknown(),
            positional: vec![],
            named: vec![],
        }
    }
}
