use std::ops::{Index, IndexMut};

use crate::{Signature, Span, VarId};

use super::Pipeline;

#[derive(Debug, Clone)]
pub struct Block {
    pub signature: Box<Signature>,
    pub pipelines: Vec<Pipeline>,
    pub captures: Vec<VarId>,
    pub redirect_env: bool,
    pub span: Option<Span>, // None option encodes no span to avoid using test_span()
}

impl Block {
    pub fn len(&self) -> usize {
        self.pipelines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pipelines.is_empty()
    }
}

impl Index<usize> for Block {
    type Output = Pipeline;

    fn index(&self, index: usize) -> &Self::Output {
        &self.pipelines[index]
    }
}

impl IndexMut<usize> for Block {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.pipelines[index]
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::new()
    }
}

impl Block {
    pub fn new() -> Self {
        Self {
            signature: Box::new(Signature::new("")),
            pipelines: vec![],
            captures: vec![],
            redirect_env: false,
            span: None,
        }
    }
}

impl<T> From<T> for Block
where
    T: Iterator<Item = Pipeline>,
{
    fn from(pipelines: T) -> Self {
        Self {
            signature: Box::new(Signature::new("")),
            pipelines: pipelines.collect(),
            captures: vec![],
            redirect_env: false,
            span: None,
        }
    }
}
