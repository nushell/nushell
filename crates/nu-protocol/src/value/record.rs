use std::ops::RangeBounds;

use crate::{ShellError, Span, Value};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Record {
    /// Don't use this field publicly!
    ///
    /// Only public as command `rename` is not reimplemented in a sane way yet
    /// Using it or making `vals` public will draw shaming by @sholderbach
    pub cols: Vec<String>,
    vals: Vec<Value>,
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
    // WARNING! Panics with assertion failure if cols and vals have different length!
    // Should be used only when the same lengths are guaranteed!
    //
    // For perf reasons does not validate the rest of the record assumptions.
    // - unique keys
    pub fn from_raw_cols_vals_unchecked(cols: Vec<String>, vals: Vec<Value>) -> Self {
        assert_eq!(cols.len(), vals.len());

        Self { cols, vals }
    }

    // Constructor that checks that `cols` and `vals` are of the same length.
    //
    // Returns None if cols and vals have different length.
    //
    // For perf reasons does not validate the rest of the record assumptions.
    // - unique keys
    pub fn from_raw_cols_vals(
        cols: Vec<String>,
        vals: Vec<Value>,
        input_span: Span,
        creation_site_span: Span,
    ) -> Result<Self, ShellError> {
        if cols.len() == vals.len() {
            Ok(Self { cols, vals })
        } else {
            Err(ShellError::RecordColsValsMismatch {
                bad_value: input_span,
                creation_site: creation_site_span,
            })
        }
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
        // `Vec::retain` is able to optimize memcopies internally.
        // For maximum benefit, `retain` is used on `vals`,
        // as `Value` is a larger struct than `String`.
        //
        // To do a simultaneous retain on the `cols`, three portions of it are tracked:
        //     [..retained, ..dropped, ..unvisited]

        // number of elements keep so far, start of ..dropped and length of ..retained
        let mut retained = 0;
        // current index of element being checked, start of ..unvisited
        let mut idx = 0;

        self.vals.retain_mut(|val| {
            if keep(&self.cols[idx], val) {
                // skip swaps for first consecutive run of kept elements
                if idx != retained {
                    self.cols.swap(idx, retained);
                }
                retained += 1;
                idx += 1;
                true
            } else {
                idx += 1;
                false
            }
        });
        self.cols.truncate(retained);
    }

    /// Truncate record to the first `len` elements.
    ///
    /// `len > self.len()` will be ignored
    /// ```rust
    /// use nu_protocol::{record, Value};
    ///
    /// let mut rec = record!(
    ///     "a" => Value::test_nothing(),
    ///     "b" => Value::test_int(42),
    ///     "c" => Value::test_nothing(),
    ///     "d" => Value::test_int(42),
    ///     );
    /// rec.truncate(42); // this is fine
    /// assert_eq!(rec.columns().map(String::as_str).collect::<String>(), "abcd");
    /// rec.truncate(2); // truncate
    /// assert_eq!(rec.columns().map(String::as_str).collect::<String>(), "ab");
    /// rec.truncate(0); // clear the record
    /// assert_eq!(rec.len(), 0);
    /// ```
    pub fn truncate(&mut self, len: usize) {
        self.cols.truncate(len);
        self.vals.truncate(len);
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

    /// Obtain an iterator to remove elements in `range`
    ///
    /// Elements not consumed from the iterator will be dropped
    ///
    /// ```rust
    /// use nu_protocol::{record, Value};
    ///
    /// let mut rec = record!(
    ///     "a" => Value::test_nothing(),
    ///     "b" => Value::test_int(42),
    ///     "c" => Value::test_string("foo"),
    /// );
    /// {
    ///     let mut drainer = rec.drain(1..);
    ///     assert_eq!(drainer.next(), Some(("b".into(), Value::test_int(42))));
    ///     // Dropping the `Drain`
    /// }
    /// let mut rec_iter = rec.into_iter();
    /// assert_eq!(rec_iter.next(), Some(("a".into(), Value::test_nothing())));
    /// assert_eq!(rec_iter.next(), None);
    /// ```
    pub fn drain<R>(&mut self, range: R) -> Drain
    where
        R: RangeBounds<usize> + Clone,
    {
        assert_eq!(
            self.cols.len(),
            self.vals.len(),
            "Length of cols and vals must be equal for sane `Record::drain`"
        );
        Drain {
            keys: self.cols.drain(range.clone()),
            values: self.vals.drain(range),
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

pub struct Drain<'a> {
    keys: std::vec::Drain<'a, String>,
    values: std::vec::Drain<'a, Value>,
}

impl Iterator for Drain<'_> {
    type Item = (String, Value);

    fn next(&mut self) -> Option<Self::Item> {
        Some((self.keys.next()?, self.values.next()?))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.keys.size_hint()
    }
}

impl DoubleEndedIterator for Drain<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        Some((self.keys.next_back()?, self.values.next_back()?))
    }
}

impl ExactSizeIterator for Drain<'_> {
    fn len(&self) -> usize {
        self.keys.len()
    }
}

#[macro_export]
macro_rules! record {
    {$($col:expr => $val:expr),+ $(,)?} => {
        $crate::Record::from_raw_cols_vals_unchecked (
            vec![$($col.into(),)+],
            vec![$($val,)+]
        )
    };
    {} => {
        $crate::Record::new()
    };
}
