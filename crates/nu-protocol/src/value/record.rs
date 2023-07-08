use crate::Value;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub cols: Vec<String>,
    pub vals: Vec<Value>,
}

impl Record {
    pub fn new() -> Self {
        Self {
            cols: vec![],
            vals: vec![],
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cols: Vec::with_capacity(capacity),
            vals: Vec::with_capacity(capacity),
        }
    }

    pub fn singleton(col: String, val: Value) -> Self {
        Self {
            cols: vec![col],
            vals: vec![val],
        }
    }

    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }

    pub fn iter_cloned(&self) -> impl DoubleEndedIterator<Item = (String, Value)> + '_ {
        self.into_iter().map(|(k, v)| (k.clone(), v.clone()))
    }

    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = (&String, &mut Value)> {
        self.cols.iter().zip(&mut self.vals)
    }

    pub fn is_empty(&self) -> bool {
        self.cols.is_empty()
    }

    pub fn len(&self) -> usize {
        self.cols.len()
    }

    pub fn push(&mut self, col: impl Into<String>, val: Value) {
        self.cols.push(col.into());
        self.vals.push(val);
    }
}

impl Default for Record {
    fn default() -> Self {
        Self::new()
    }
}

impl FromIterator<(String, Value)> for Record {
    fn from_iter<T: IntoIterator<Item = (String, Value)>>(iter: T) -> Self {
        let (cols, vals) = iter.into_iter().unzip();
        Self { cols, vals }
    }
}

impl IntoIterator for Record {
    type Item = (String, Value);

    type IntoIter = std::iter::Zip<std::vec::IntoIter<String>, std::vec::IntoIter<Value>>;

    fn into_iter(self) -> Self::IntoIter {
        self.cols.into_iter().zip(self.vals)
    }
}

impl<'a> IntoIterator for &'a Record {
    type Item = (&'a String, &'a Value);

    type IntoIter = std::iter::Zip<std::slice::Iter<'a, String>, std::slice::Iter<'a, Value>>;

    fn into_iter(self) -> Self::IntoIter {
        self.cols.iter().zip(&self.vals)
    }
}

#[macro_export]
macro_rules! record {
    {$($col:ident => $val:expr),+ $(,)?} => {
        $crate::Record {
            cols: vec![$(stringify!($col).to_string(),)+],
            vals: vec![$($val,)+]
        }

    };
    {$($col:expr => $val:expr),+ $(,)?} => {
        $crate::Record {
            cols: vec![$($col.into(),)+],
            vals: vec![$($val,)+]
        }

    };
    {$($col:ident),+ $(,)?} => {
        $crate::Record {
            cols: vec![$(stringify!($col).to_string(),)+],
            vals: vec![$($col,)+]
        }
    };
    {} => {
        $crate::Record::new()
    };
}
