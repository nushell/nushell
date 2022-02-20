use std::ops::{Index, IndexMut};

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

    pub fn len(&self) -> usize {
        self.expressions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.expressions.is_empty()
    }
}

impl Index<usize> for Pipeline {
    type Output = Expression;

    fn index(&self, index: usize) -> &Self::Output {
        &self.expressions[index]
    }
}

impl IndexMut<usize> for Pipeline {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.expressions[index]
    }
}
