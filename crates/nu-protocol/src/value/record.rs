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
        if let Some(idx) = self.index_of(&col) {
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
        self.columns().position(|k| k == col.as_ref())
    }

    pub fn get(&self, col: impl AsRef<str>) -> Option<&Value> {
        self.index_of(col).and_then(|idx| self.vals.get(idx))
    }

    pub fn get_mut(&mut self, col: impl AsRef<str>) -> Option<&mut Value> {
        self.index_of(col).and_then(|idx| self.vals.get_mut(idx))
    }

    pub fn get_index(&self, idx: usize) -> Option<(&String, &Value)> {
        Some((self.cols.get(idx)?, self.vals.get(idx)?))
    }

    /// Remove single value by key
    ///
    /// Returns `None` if key not found
    ///
    /// Note: makes strong assumption that keys are unique
    pub fn remove(&mut self, col: impl AsRef<str>) -> Option<Value> {
        let idx = self.index_of(col)?;
        self.cols.remove(idx);
        Some(self.vals.remove(idx))
    }

    /// Remove elements in-place that do not satisfy `keep`
    ///
    /// Note: Panics if `vals.len() > cols.len()`
    /// ```rust
    /// use nu_protocol::{record, Value};
    ///
    /// let mut rec = record!(
    ///     "a" => Value::test_nothing(),
    ///     "b" => Value::test_int(42),
    ///     "c" => Value::test_nothing(),
    ///     "d" => Value::test_int(42),
    ///     );
    /// rec.retain(|_k, val| !val.is_nothing());
    /// let mut iter_rec = rec.columns();
    /// assert_eq!(iter_rec.next().map(String::as_str), Some("b"));
    /// assert_eq!(iter_rec.next().map(String::as_str), Some("d"));
    /// assert_eq!(iter_rec.next(), None);
    /// ```
    pub fn retain<F>(&mut self, mut keep: F)
    where
        F: FnMut(&str, &Value) -> bool,
    {
        self.retain_mut(|k, v| keep(k, v));
    }

    /// Remove elements in-place that do not satisfy `keep` while allowing mutation of the value.
    ///
    /// This can for example be used to recursively prune nested records.
    ///
    /// Note: Panics if `vals.len() > cols.len()`
    /// ```rust
    /// use nu_protocol::{record, Record, Value};
    ///
    /// fn remove_foo_recursively(val: &mut Value) {
    ///     if let Value::Record {val, ..} = val {
    ///         val.retain_mut(keep_non_foo);
    ///     }
    /// }
    ///
    /// fn keep_non_foo(k: &str, v: &mut Value) -> bool {
    ///     if k == "foo" {
    ///         return false;
    ///     }
    ///     remove_foo_recursively(v);
    ///     true
    /// }
    ///
    /// let mut test = Value::test_record(record!(
    ///     "foo" => Value::test_nothing(),
    ///     "bar" => Value::test_record(record!(
    ///         "foo" => Value::test_nothing(),
    ///         "baz" => Value::test_nothing(),
    ///         ))
    ///     ));
    ///
    /// remove_foo_recursively(&mut test);
    /// let expected = Value::test_record(record!(
    ///     "bar" => Value::test_record(record!(
    ///         "baz" => Value::test_nothing(),
    ///         ))
    ///     ));
    /// assert_eq!(test, expected);
    /// ```
    pub fn retain_mut<F>(&mut self, mut keep: F)
    where
        F: FnMut(&str, &mut Value) -> bool,
    {
        let mut idx = 0;

        // `Vec::retain` is able to optimize memcopies internally. For maximum benefit as `Value`
        // is a larger struct than `String` use `retain` on `vals`
        //
        // The calls to `Vec::remove` are suboptimal as they need memcopies to shift each time.
        //
        // As the operations should remain inplace, we don't allocate a separate index `Vec` which
        // could be used to avoid the repeated shifting of `Vec::remove` in cols.
        self.vals.retain_mut(|val| {
            if keep(self.cols[idx].as_str(), val) {
                idx += 1;
                true
            } else {
                self.cols.remove(idx);
                false
            }
        });
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
