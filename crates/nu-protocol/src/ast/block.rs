use std::ops::{Index, IndexMut};

use crate::{Overlay, Signature, VarId};

use super::Statement;

#[derive(Debug, Clone)]
pub struct Block {
    pub signature: Box<Signature>,
    pub stmts: Vec<Statement>,
    pub overlay: Overlay,
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
            overlay: Overlay::new(),
            captures: vec![],
        }
    }

    pub fn with_overlay(self, overlay: Overlay) -> Self {
        Self {
            signature: self.signature,
            stmts: self.stmts,
            overlay,
            captures: self.captures,
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
            overlay: Overlay::new(),
            captures: vec![],
        }
    }
}
