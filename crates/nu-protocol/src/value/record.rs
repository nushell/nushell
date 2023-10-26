use crate::Value;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Record {
    pub cols: Vec<String>,
    pub vals: Vec<Value>,
}

impl Record {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cols: Vec::with_capacity(capacity),
            vals: Vec::with_capacity(capacity),
        }
    }

    // Constructor that checks that `cols` and `vals` are of the same length.
    //
    // For perf reasons does not validate the rest of the record assumptions.
    // - unique keys
    pub fn from_raw_cols_vals(cols: Vec<String>, vals: Vec<Value>) -> Self {
        assert_eq!(cols.len(), vals.len());

        Self { cols, vals }
    }

    pub fn iter(&self) -> Iter {
        self.into_iter()
    }

    pub fn iter_mut(&mut self) -> IterMut {
        self.into_iter()
    }

    pub fn is_empty(&self) -> bool {
        self.cols.is_empty() || self.vals.is_empty()
    }

    pub fn len(&self) -> usize {
        usize::min(self.cols.len(), self.vals.len())
    }

    /// Naive push to the end of the datastructure.
    ///
    /// May duplicate data!
    ///
    /// Consider to use [`Record::insert`] instead
    pub fn push(&mut self, col: impl Into<String>, val: Value) {
        self.cols.push(col.into());
        self.vals.push(val);
    }

    /// Insert into the record, replacing preexisting value if found.
    ///
    /// Returns `Some(previous_value)` if found. Else `None`
    pub fn insert<K>(&mut self, col: K, val: Value) -> Option<Value>
    where
        K: AsRef<str> + Into<String>,
    {
        if let Some(idx) = self.columns().position(|k| k == col.as_ref()) {
            // Can panic if vals.len() < cols.len()
            let curr_val = &mut self.vals[idx];
            Some(std::mem::replace(curr_val, val))
        } else {
            self.cols.push(col.into());
            self.vals.push(val);
            None
        }
    }

    pub fn contains(&self, col: impl AsRef<str>) -> bool {
        self.cols.iter().any(|k| k == col.as_ref())
    }

    pub fn index_of(&self, col: impl AsRef<str>) -> Option<usize> {
        self.columns()
            .position(|k| k == col.as_ref())
    }

    pub fn get(&self, col: impl AsRef<str>) -> Option<&Value> {
        self.columns()
            .position(|k| k == col.as_ref())
            .and_then(|idx| self.vals.get(idx))
    }

    pub fn get_mut(&mut self, col: impl AsRef<str>) -> Option<&mut Value> {
        self.columns()
            .position(|k| k == col.as_ref())
            .and_then(|idx| self.vals.get_mut(idx))
    }

    pub fn get_index(&self, idx: usize) -> Option<(&String, &Value)> {
        Some((self.cols.get(idx)?, self.vals.get(idx)?))
    }

    pub fn columns(&self) -> Columns {
        Columns {
            iter: self.cols.iter(),
        }
    }

    pub fn values(&self) -> Values {
        Values {
            iter: self.vals.iter(),
        }
    }

    pub fn into_values(self) -> IntoValues {
        IntoValues {
            iter: self.vals.into_iter(),
        }
    }
}

impl FromIterator<(String, Value)> for Record {
    fn from_iter<T: IntoIterator<Item = (String, Value)>>(iter: T) -> Self {
        let (cols, vals) = iter.into_iter().unzip();
        Self { cols, vals }
    }
}

impl Extend<(String, Value)> for Record {
    fn extend<T: IntoIterator<Item = (String, Value)>>(&mut self, iter: T) {
        for (k, v) in iter {
            // TODO: should this .insert with a check?
            self.cols.push(k);
            self.vals.push(v);
        }
    }
}

pub type IntoIter = std::iter::Zip<std::vec::IntoIter<String>, std::vec::IntoIter<Value>>;

impl IntoIterator for Record {
    type Item = (String, Value);

    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.cols.into_iter().zip(self.vals)
    }
}

pub type Iter<'a> = std::iter::Zip<std::slice::Iter<'a, String>, std::slice::Iter<'a, Value>>;

impl<'a> IntoIterator for &'a Record {
    type Item = (&'a String, &'a Value);

    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.cols.iter().zip(&self.vals)
    }
}

pub type IterMut<'a> = std::iter::Zip<std::slice::Iter<'a, String>, std::slice::IterMut<'a, Value>>;

impl<'a> IntoIterator for &'a mut Record {
    type Item = (&'a String, &'a mut Value);

    type IntoIter = IterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.cols.iter().zip(&mut self.vals)
    }
}

pub struct Columns<'a> {
    iter: std::slice::Iter<'a, String>,
}

impl<'a> Iterator for Columns<'a> {
    type Item = &'a String;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a> DoubleEndedIterator for Columns<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

impl<'a> ExactSizeIterator for Columns<'a> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

pub struct Values<'a> {
    iter: std::slice::Iter<'a, Value>,
}

impl<'a> Iterator for Values<'a> {
    type Item = &'a Value;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a> DoubleEndedIterator for Values<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

impl<'a> ExactSizeIterator for Values<'a> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

pub struct IntoValues {
    iter: std::vec::IntoIter<Value>,
}

impl Iterator for IntoValues {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for IntoValues {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

impl ExactSizeIterator for IntoValues {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

#[macro_export]
macro_rules! record {
    {$($col:expr => $val:expr),+ $(,)?} => {
        $crate::Record::from_raw_cols_vals (
            vec![$($col.into(),)+],
            vec![$($val,)+]
        )
    };
    {} => {
        $crate::Record::new()
    };
}
