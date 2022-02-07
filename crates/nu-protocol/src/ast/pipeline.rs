use crate::ast::Expression;

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub expressions: Vec<Expression>,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            expressions: vec![],
        }
    }

    pub fn from_vec(expressions: Vec<Expression>) -> Pipeline {
        Self { expressions }
    }
}
