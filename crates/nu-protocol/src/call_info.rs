use crate::value::Value;
use derive_new::new;
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_source::Tag;
use serde::{Deserialize, Serialize};

/// Associated information for the call of a command, including the args passed to the command and a tag that spans the name of the command being called
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CallInfo {
    /// The arguments associated with this call
    pub args: EvaluatedArgs,
    /// The tag (underline-able position) of the name of the call itself
    pub name_tag: Tag,
}

/// The set of positional and named arguments, after their values have been evaluated.
///
/// * Positional arguments are those who are given as values, without any associated flag. For example, in `foo arg1 arg2`, both `arg1` and `arg2` are positional arguments.
/// * Named arguments are those associated with a flag. For example, `foo --given bar` the named argument would be name `given` and the value `bar`.
#[derive(Debug, Default, new, Serialize, Deserialize, Clone)]
pub struct EvaluatedArgs {
    pub positional: Option<Vec<Value>>,
    pub named: Option<IndexMap<String, Value>>,
}

impl EvaluatedArgs {
    /// Retrieve a subset of positional arguments starting at a given position
    pub fn slice_from(&self, from: usize) -> Vec<Value> {
        let positional = &self.positional;

        match positional {
            None => vec![],
            Some(list) => list[from..].to_vec(),
        }
    }

    /// Get the nth positional argument, if possible
    pub fn nth(&self, pos: usize) -> Option<&Value> {
        match &self.positional {
            None => None,
            Some(array) => array.get(pos),
        }
    }

    /// Get the nth positional argument, error if not possible
    pub fn expect_nth(&self, pos: usize) -> Result<&Value, ShellError> {
        match &self.positional {
            None => Err(ShellError::unimplemented("Better error: expect_nth")),
            Some(array) => match array.get(pos) {
                None => Err(ShellError::unimplemented("Better error: expect_nth")),
                Some(item) => Ok(item),
            },
        }
    }

    /// Get the number of positional arguments available
    pub fn len(&self) -> usize {
        match &self.positional {
            None => 0,
            Some(array) => array.len(),
        }
    }

    /// Return if there are no positional arguments
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return true if the set of named arguments contains the name provided
    pub fn has(&self, name: &str) -> bool {
        matches!(&self.named, Some(named) if named.contains_key(name))
    }

    /// Gets the corresponding Value for the named argument given, if possible
    pub fn get(&self, name: &str) -> Option<&Value> {
        match &self.named {
            None => None,
            Some(named) => named.get(name),
        }
    }

    /// Iterates over the positional arguments
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

/// An iterator to help iterate over positional arguments
pub enum PositionalIter<'a> {
    Empty,
    Array(std::slice::Iter<'a, Value>),
}

impl<'a> Iterator for PositionalIter<'a> {
    type Item = &'a Value;

    /// The required `next` function to implement the Iterator trait
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            PositionalIter::Empty => None,
            PositionalIter::Array(iter) => iter.next(),
        }
    }
}
