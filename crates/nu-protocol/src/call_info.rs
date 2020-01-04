use crate::value::Value;
use derive_new::new;
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_source::Tag;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CallInfo {
    pub args: EvaluatedArgs,
    pub name_tag: Tag,
}

#[derive(Debug, Default, new, Serialize, Deserialize, Clone)]
pub struct EvaluatedArgs {
    pub positional: Option<Vec<Value>>,
    pub named: Option<IndexMap<String, Value>>,
}

impl EvaluatedArgs {
    pub fn slice_from(&self, from: usize) -> Vec<Value> {
        let positional = &self.positional;

        match positional {
            None => vec![],
            Some(list) => list[from..].to_vec(),
        }
    }

    pub fn nth(&self, pos: usize) -> Option<&Value> {
        match &self.positional {
            None => None,
            Some(array) => array.get(pos),
        }
    }

    pub fn expect_nth(&self, pos: usize) -> Result<&Value, ShellError> {
        match &self.positional {
            None => Err(ShellError::unimplemented("Better error: expect_nth")),
            Some(array) => match array.get(pos) {
                None => Err(ShellError::unimplemented("Better error: expect_nth")),
                Some(item) => Ok(item),
            },
        }
    }

    pub fn len(&self) -> usize {
        match &self.positional {
            None => 0,
            Some(array) => array.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn has(&self, name: &str) -> bool {
        match &self.named {
            None => false,
            Some(named) => named.contains_key(name),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        match &self.named {
            None => None,
            Some(named) => named.get(name),
        }
    }

    pub fn positional_iter(&self) -> PositionalIter<'_> {
        match &self.positional {
            None => PositionalIter::Empty,
            Some(v) => {
                let iter = v.iter();
                PositionalIter::Array(iter)
            }
        }
    }
}

pub enum PositionalIter<'a> {
    Empty,
    Array(std::slice::Iter<'a, Value>),
}

impl<'a> Iterator for PositionalIter<'a> {
    type Item = &'a Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            PositionalIter::Empty => None,
            PositionalIter::Array(iter) => iter.next(),
        }
    }
}
