use std::ops::{Index, IndexMut};

use crate::{Signature, VarId};

use super::Statement;

#[derive(Debug, Clone)]
pub struct Block {
    pub signature: Box<Signature>,
    pub stmts: Vec<Statement>,
    pub captures: Vec<VarId>,
}

impl Block {
    pub fn len(&self) -> usize {
        self.stmts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stmts.is_empty()
    }
}

impl Index<usize> for Block {
    type Output = Statement;

    fn index(&self, index: usize) -> &Self::Output {
        &self.stmts[index]
    }
}

impl IndexMut<usize> for Block {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.stmts[index]
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
            stmts: vec![],
            captures: vec![],
        }
    }
}

impl<T> From<T> for Block
where
    T: Iterator<Item = Statement>,
{
    fn from(stmts: T) -> Self {
        Self {
            signature: Box::new(Signature::new("")),
            stmts: stmts.collect(),
            captures: vec![],
        }
    }
}
