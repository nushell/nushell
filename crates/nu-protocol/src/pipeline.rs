use crate::Expression;

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
}
