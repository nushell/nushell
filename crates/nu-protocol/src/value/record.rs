use std::{iter::FusedIterator, ops::RangeBounds};

use crate::{ShellError, Span, Value};

use serde::{de::Visitor, ser::SerializeMap, Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub struct Record {
    inner: Vec<(String, Value)>,
}

impl Record {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Vec::with_capacity(capacity),
        }
    }

    /// Create a [`Record`] from a `Vec` of columns and a `Vec` of [`Value`]s
    ///
    /// Returns an error if `cols` and `vals` have different lengths.
    ///
    /// For perf reasons, this will not validate the rest of the record assumptions:
    /// - unique keys
    pub fn from_raw_cols_vals(
        cols: Vec<String>,
        vals: Vec<Value>,
        input_span: Span,
        creation_site_span: Span,
    ) -> Result<Self, ShellError> {
        if cols.len() == vals.len() {
            let inner = cols.into_iter().zip(vals).collect();
            Ok(Self { inner })
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
        self.inner.is_empty()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Naive push to the end of the datastructure.
    ///
    /// May duplicate data!
    ///
    /// Consider to use [`Record::insert`] instead
    pub fn push(&mut self, col: impl Into<String>, val: Value) {
        self.inner.push((col.into(), val));
    }

    /// Insert into the record, replacing preexisting value if found.
    ///
    /// Returns `Some(previous_value)` if found. Else `None`
    pub fn insert<K>(&mut self, col: K, val: Value) -> Option<Value>
    where
        K: AsRef<str> + Into<String>,
    {
        if let Some(curr_val) = self.get_mut(&col) {
            Some(std::mem::replace(curr_val, val))
        } else {
            self.push(col, val);
            None
        }
    }

    pub fn contains(&self, col: impl AsRef<str>) -> bool {
        self.columns().any(|k| k == col.as_ref())
    }

    pub fn index_of(&self, col: impl AsRef<str>) -> Option<usize> {
        self.columns().position(|k| k == col.as_ref())
    }

    pub fn get(&self, col: impl AsRef<str>) -> Option<&Value> {
        self.inner
            .iter()
            .find_map(|(k, v)| if k == col.as_ref() { Some(v) } else { None })
    }

    pub fn get_mut(&mut self, col: impl AsRef<str>) -> Option<&mut Value> {
        self.inner
            .iter_mut()
            .find_map(|(k, v)| if k == col.as_ref() { Some(v) } else { None })
    }

    pub fn get_index(&self, idx: usize) -> Option<(&String, &Value)> {
        self.inner.get(idx).map(|(col, val): &(_, _)| (col, val))
    }

    /// Remove single value by key
    ///
    /// Returns `None` if key not found
    ///
    /// Note: makes strong assumption that keys are unique
    pub fn remove(&mut self, col: impl AsRef<str>) -> Option<Value> {
        let idx = self.index_of(col)?;
        let (_, val) = self.inner.remove(idx);
        Some(val)
    }

    /// Remove elements in-place that do not satisfy `keep`
    ///
    /// ```rust
    /// use nu_protocol::{record, Value};
    ///
    /// let mut rec = record!(
    ///     "a" => Value::test_nothing(),
    ///     "b" => Value::test_int(42),
    ///     "c" => Value::test_nothing(),
    ///     "d" => Value::test_int(42),
    /// );
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
    /// ```rust
    /// use nu_protocol::{record, Record, Value};
    ///
    /// fn remove_foo_recursively(val: &mut Value) {
    ///     if let Value::Record {val, ..} = val {
    ///         val.to_mut().retain_mut(keep_non_foo);
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
        self.inner.retain_mut(|(col, val)| keep(col, val));
    }

    /// Truncate record to the first `len` elements.
    ///
    /// `len > self.len()` will be ignored
    ///
    /// ```rust
    /// use nu_protocol::{record, Value};
    ///
    /// let mut rec = record!(
    ///     "a" => Value::test_nothing(),
    ///     "b" => Value::test_int(42),
    ///     "c" => Value::test_nothing(),
    ///     "d" => Value::test_int(42),
    /// );
    /// rec.truncate(42); // this is fine
    /// assert_eq!(rec.columns().map(String::as_str).collect::<String>(), "abcd");
    /// rec.truncate(2); // truncate
    /// assert_eq!(rec.columns().map(String::as_str).collect::<String>(), "ab");
    /// rec.truncate(0); // clear the record
    /// assert_eq!(rec.len(), 0);
    /// ```
    pub fn truncate(&mut self, len: usize) {
        self.inner.truncate(len);
    }

    pub fn columns(&self) -> Columns {
        Columns {
            iter: self.inner.iter(),
        }
    }

    pub fn into_columns(self) -> IntoColumns {
        IntoColumns {
            iter: self.inner.into_iter(),
        }
    }

    pub fn values(&self) -> Values {
        Values {
            iter: self.inner.iter(),
        }
    }

    pub fn into_values(self) -> IntoValues {
        IntoValues {
            iter: self.inner.into_iter(),
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
        Drain {
            iter: self.inner.drain(range),
        }
    }

    /// Sort the record by its columns.
    ///
    /// ```rust
    /// use nu_protocol::{record, Value};
    ///
    /// let mut rec = record!(
    ///     "c" => Value::test_string("foo"),
    ///     "b" => Value::test_int(42),
    ///     "a" => Value::test_nothing(),
    /// );
    ///
    /// rec.sort_cols();
    ///
    /// assert_eq!(
    ///     Value::test_record(rec),
    ///     Value::test_record(record!(
    ///         "a" => Value::test_nothing(),
    ///         "b" => Value::test_int(42),
    ///         "c" => Value::test_string("foo"),
    ///     ))
    /// );
    /// ```
    pub fn sort_cols(&mut self) {
        self.inner.sort_by(|(k1, _), (k2, _)| k1.cmp(k2))
    }
}

impl Serialize for Record {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (k, v) in self {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for Record {
    /// Special deserialization implementation that turns a map-pattern into a [`Record`]
    ///
    /// Denies duplicate keys
    ///
    /// ```rust
    /// use serde_json::{from_str, Result};
    /// use nu_protocol::{Record, Value, record};
    ///
    /// // A `Record` in json is a Record with a packed `Value`
    /// // The `Value` record has a single key indicating its type and the inner record describing
    /// // its representation of value and the associated `Span`
    /// let ok = r#"{"a": {"Int": {"val": 42, "span": {"start": 0, "end": 0}}},
    ///              "b": {"Int": {"val": 37, "span": {"start": 0, "end": 0}}}}"#;
    /// let ok_rec: Record = from_str(ok).unwrap();
    /// assert_eq!(Value::test_record(ok_rec),
    ///            Value::test_record(record!{"a" => Value::test_int(42),
    ///                                       "b" => Value::test_int(37)}));
    /// // A repeated key will lead to a deserialization error
    /// let bad = r#"{"a": {"Int": {"val": 42, "span": {"start": 0, "end": 0}}},
    ///               "a": {"Int": {"val": 37, "span": {"start": 0, "end": 0}}}}"#;
    /// let bad_rec: Result<Record> = from_str(bad);
    /// assert!(bad_rec.is_err());
    /// ```
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(RecordVisitor)
    }
}

struct RecordVisitor;

impl<'de> Visitor<'de> for RecordVisitor {
    type Value = Record;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a nushell `Record` mapping string keys/columns to nushell `Value`")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut record = Record::with_capacity(map.size_hint().unwrap_or(0));

        while let Some((key, value)) = map.next_entry::<String, Value>()? {
            if record.insert(key, value).is_some() {
                return Err(serde::de::Error::custom(
                    "invalid entry, duplicate keys are not allowed for `Record`",
                ));
            }
        }

        Ok(record)
    }
}

impl FromIterator<(String, Value)> for Record {
    fn from_iter<T: IntoIterator<Item = (String, Value)>>(iter: T) -> Self {
        // TODO: should this check for duplicate keys/columns?
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl Extend<(String, Value)> for Record {
    fn extend<T: IntoIterator<Item = (String, Value)>>(&mut self, iter: T) {
        for (k, v) in iter {
            // TODO: should this .insert with a check?
            self.push(k, v)
        }
    }
}

pub struct IntoIter {
    iter: std::vec::IntoIter<(String, Value)>,
}

impl Iterator for IntoIter {
    type Item = (String, Value);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for IntoIter {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

impl ExactSizeIterator for IntoIter {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl FusedIterator for IntoIter {}

impl IntoIterator for Record {
    type Item = (String, Value);

    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            iter: self.inner.into_iter(),
        }
    }
}

pub struct Iter<'a> {
    iter: std::slice::Iter<'a, (String, Value)>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a String, &'a Value);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(col, val): &(_, _)| (col, val))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a> DoubleEndedIterator for Iter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|(col, val): &(_, _)| (col, val))
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl FusedIterator for Iter<'_> {}

impl<'a> IntoIterator for &'a Record {
    type Item = (&'a String, &'a Value);

    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            iter: self.inner.iter(),
        }
    }
}

pub struct IterMut<'a> {
    iter: std::slice::IterMut<'a, (String, Value)>,
}

impl<'a> Iterator for IterMut<'a> {
    type Item = (&'a String, &'a mut Value);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(col, val)| (&*col, val))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a> DoubleEndedIterator for IterMut<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|(col, val)| (&*col, val))
    }
}

impl<'a> ExactSizeIterator for IterMut<'a> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl FusedIterator for IterMut<'_> {}

impl<'a> IntoIterator for &'a mut Record {
    type Item = (&'a String, &'a mut Value);

    type IntoIter = IterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        IterMut {
            iter: self.inner.iter_mut(),
        }
    }
}

pub struct Columns<'a> {
    iter: std::slice::Iter<'a, (String, Value)>,
}

impl<'a> Iterator for Columns<'a> {
    type Item = &'a String;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(col, _)| col)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a> DoubleEndedIterator for Columns<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|(col, _)| col)
    }
}

impl<'a> ExactSizeIterator for Columns<'a> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl FusedIterator for Columns<'_> {}

pub struct IntoColumns {
    iter: std::vec::IntoIter<(String, Value)>,
}

impl Iterator for IntoColumns {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(col, _)| col)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for IntoColumns {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|(col, _)| col)
    }
}

impl ExactSizeIterator for IntoColumns {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl FusedIterator for IntoColumns {}

pub struct Values<'a> {
    iter: std::slice::Iter<'a, (String, Value)>,
}

impl<'a> Iterator for Values<'a> {
    type Item = &'a Value;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(_, val)| val)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a> DoubleEndedIterator for Values<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|(_, val)| val)
    }
}

impl<'a> ExactSizeIterator for Values<'a> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl FusedIterator for Values<'_> {}

pub struct IntoValues {
    iter: std::vec::IntoIter<(String, Value)>,
}

impl Iterator for IntoValues {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(_, val)| val)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for IntoValues {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|(_, val)| val)
    }
}

impl ExactSizeIterator for IntoValues {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl FusedIterator for IntoValues {}

pub struct Drain<'a> {
    iter: std::vec::Drain<'a, (String, Value)>,
}

impl Iterator for Drain<'_> {
    type Item = (String, Value);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for Drain<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

impl ExactSizeIterator for Drain<'_> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl FusedIterator for Drain<'_> {}

#[macro_export]
macro_rules! record {
    // The macro only compiles if the number of columns equals the number of values,
    // so it's safe to call `unwrap` below.
    {$($col:expr => $val:expr),+ $(,)?} => {
        $crate::Record::from_raw_cols_vals(
            ::std::vec![$($col.into(),)+],
            ::std::vec![$($val,)+],
            $crate::Span::unknown(),
            $crate::Span::unknown(),
        ).unwrap()
    };
    {} => {
        $crate::Record::new()
    };
}
