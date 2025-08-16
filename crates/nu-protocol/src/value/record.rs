//! Our insertion ordered map-type [`Record`]
use std::iter::FusedIterator;

use crate::{ShellError, Span, Value, casing::Casing};

use ecow::EcoVec;
use nu_utils::IgnoreCaseExt;
use serde::{Deserialize, Serialize, de::Visitor, ser::SerializeMap};

#[derive(Debug, Clone, Default)]
pub struct Record {
    cols: EcoVec<String>,
    vals: EcoVec<Value>,
}

#[repr(transparent)]
pub struct RecordTemplate(EcoVec<String>);

impl Record {
    pub fn new_template<I>(cols: I) -> RecordTemplate
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        RecordTemplate(cols.into_iter().map(Into::into).collect())
    }
}

impl RecordTemplate {
    // TODO: Use a proper error type
    pub fn add_values<I>(&self, vals: I) -> Record
    where
        I: IntoIterator<Item = Value>,
    {
        let vals: EcoVec<Value> = vals.into_iter().collect();
        debug_assert_eq!(self.len(), vals.len());
        let cols = self.0.clone();
        Record { cols, vals }
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A wrapper around [`Record`] that affects whether key comparisons are case sensitive or not.
///
/// Implements commonly used methods of [`Record`].
pub struct CasedRecord<R> {
    record: R,
    casing: Casing,
}

impl<R> CasedRecord<R> {
    fn cmp(&self, lhs: &str, rhs: &str) -> bool {
        match self.casing {
            Casing::Sensitive => lhs == rhs,
            Casing::Insensitive => lhs.eq_ignore_case(rhs),
        }
    }
}

impl<'a> CasedRecord<&'a Record> {
    pub fn contains(&self, col: impl AsRef<str>) -> bool {
        self.record.columns().any(|k| self.cmp(k, col.as_ref()))
    }

    pub fn index_of(&self, col: impl AsRef<str>) -> Option<usize> {
        self.record
            .columns()
            .rposition(|k| self.cmp(k, col.as_ref()))
    }

    pub fn get(self, col: impl AsRef<str>) -> Option<&'a Value> {
        let idx = self.index_of(col)?;
        let (_, value) = self.record.get_index(idx)?;
        Some(value)
    }
}

impl<'a> CasedRecord<&'a mut Record> {
    fn shared(&'a self) -> CasedRecord<&'a Record> {
        CasedRecord {
            record: &*self.record,
            casing: self.casing,
        }
    }

    pub fn get_mut(self, col: impl AsRef<str>) -> Option<&'a mut Value> {
        let idx = self.shared().index_of(col)?;
        let (_, value) = self.record.get_index_mut(idx)?;
        Some(value)
    }

    pub fn remove(&mut self, col: impl AsRef<str>) -> Option<Value> {
        let idx = self.shared().index_of(col)?;
        let val = self.record.remove_index(idx);
        Some(val)
    }
}

impl Record {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cols: EcoVec::with_capacity(capacity),
            vals: EcoVec::with_capacity(capacity),
        }
    }

    pub fn cased(&self, casing: Casing) -> CasedRecord<&Record> {
        CasedRecord {
            record: self,
            casing,
        }
    }

    pub fn cased_mut(&mut self, casing: Casing) -> CasedRecord<&mut Record> {
        CasedRecord {
            record: self,
            casing,
        }
    }

    /// Create a [`Record`] from a `Vec` of columns and a `Vec` of [`Value`]s
    ///
    /// Returns an error if `cols` and `vals` have different lengths.
    ///
    /// For perf reasons, this will not validate the rest of the record assumptions:
    /// - unique keys
    pub fn from_raw_cols_vals(
        cols: impl Into<EcoVec<String>>,
        vals: impl Into<EcoVec<Value>>,
        input_span: Span,
        creation_site_span: Span,
    ) -> Result<Self, ShellError> {
        let cols = cols.into();
        let vals = vals.into();
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
        self.vals.is_empty()
    }

    pub fn len(&self) -> usize {
        self.vals.len()
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
        let idx = self.index_of(col)?;
        self.get_index(idx).map(|(_, val)| val)
    }

    pub fn get_mut(&mut self, col: impl AsRef<str>) -> Option<&mut Value> {
        let idx = self.index_of(col)?;
        self.get_index_mut(idx).map(|(_, val)| val)
    }

    pub fn get_index(&self, idx: usize) -> Option<(&String, &Value)> {
        let Self { cols, vals } = self;
        cols.get(idx).zip(vals.get(idx))
    }

    pub fn get_index_mut(&mut self, idx: usize) -> Option<(&mut String, &mut Value)> {
        let Self { cols, vals } = self;
        cols.make_mut()
            .get_mut(idx)
            .zip(vals.make_mut().get_mut(idx))
    }

    /// Remove single value by key
    ///
    /// Returns `None` if key not found
    ///
    /// Note: makes strong assumption that keys are unique
    pub fn remove(&mut self, col: impl AsRef<str>) -> Option<Value> {
        let idx = self.index_of(col)?;
        Some(self.remove_index(idx))
    }

    fn remove_index(&mut self, idx: usize) -> Value {
        self.cols.remove(idx);
        self.vals.remove(idx)
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
        let Self { cols, vals } = self;

        let keep_idx = cols
            .iter()
            .map(AsRef::<str>::as_ref)
            .zip(vals.make_mut().iter_mut())
            .map(|(k, v)| keep(k, v))
            .enumerate()
            .filter_map(|(idx, keep)| keep.then_some(idx))
            .collect::<Vec<_>>();

        fn make_keep_fn<T>(keep_idx: &[usize]) -> impl FnMut(&mut T) -> bool {
            let mut idx = 0usize;
            let mut keep_idx_iter = keep_idx.iter().peekable();

            move |_| {
                let keep = match keep_idx_iter.peek() {
                    Some(&k) if k == &idx => {
                        let _ = keep_idx_iter.next();
                        true
                    }
                    _ => false,
                };

                idx += 1;
                keep
            }
        }

        cols.retain(make_keep_fn(&keep_idx));
        vals.retain(make_keep_fn(&keep_idx));
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
        self.cols.truncate(len);
        self.vals.truncate(len);
    }

    pub fn columns(&self) -> Columns {
        Columns {
            iter: self.cols.iter(),
        }
    }

    pub fn into_columns(self) -> IntoColumns {
        IntoColumns {
            iter: self.cols.into_iter(),
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

    // /// Obtain an iterator to remove elements in `range`
    // ///
    // /// Elements not consumed from the iterator will be dropped
    // ///
    // /// ```rust
    // /// use nu_protocol::{record, Value};
    // ///
    // /// let mut rec = record!(
    // ///     "a" => Value::test_nothing(),
    // ///     "b" => Value::test_int(42),
    // ///     "c" => Value::test_string("foo"),
    // /// );
    // /// {
    // ///     let mut drainer = rec.drain(1..);
    // ///     assert_eq!(drainer.next(), Some(("b".into(), Value::test_int(42))));
    // ///     // Dropping the `Drain`
    // /// }
    // /// let mut rec_iter = rec.into_iter();
    // /// assert_eq!(rec_iter.next(), Some(("a".into(), Value::test_nothing())));
    // /// assert_eq!(rec_iter.next(), None);
    // /// ```
    // pub fn drain<R>(&mut self, range: R) -> Drain
    // where
    //     R: RangeBounds<usize> + Clone,
    // {
    //     Drain {
    //         iter: self.inner.drain(range)
    //     }
    // }

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
        let cols = self.cols.make_mut();
        let vals = self.vals.make_mut();

        let mut perm = permutation::sort(&*cols);
        perm.apply_slice_in_place(cols);
        perm.apply_slice_in_place(vals);
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
        let (cols, vals) = iter.into_iter().unzip();
        Self { cols, vals }
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
    iter: std::iter::Zip<ecow::vec::IntoIter<String>, ecow::vec::IntoIter<Value>>,
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
        let Self { cols, vals } = self;
        IntoIter {
            iter: cols.into_iter().zip(vals),
        }
    }
}

pub struct Iter<'a> {
    iter: std::iter::Zip<std::slice::Iter<'a, String>, std::slice::Iter<'a, Value>>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a String, &'a Value);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for Iter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

impl ExactSizeIterator for Iter<'_> {
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
            iter: self.cols.iter().zip(self.vals.iter()),
        }
    }
}

pub struct IterMut<'a> {
    iter: std::iter::Zip<std::slice::IterMut<'a, String>, std::slice::IterMut<'a, Value>>,
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

impl DoubleEndedIterator for IterMut<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|(col, val)| (&*col, val))
    }
}

impl ExactSizeIterator for IterMut<'_> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl FusedIterator for IterMut<'_> {}

impl<'a> IntoIterator for &'a mut Record {
    type Item = (&'a String, &'a mut Value);

    type IntoIter = IterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        let Record { cols, vals } = self;
        IterMut {
            iter: cols.make_mut().iter_mut().zip(vals.make_mut().iter_mut()),
        }
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

impl DoubleEndedIterator for Columns<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

impl ExactSizeIterator for Columns<'_> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl FusedIterator for Columns<'_> {}

pub struct IntoColumns {
    iter: ecow::vec::IntoIter<String>,
}

impl Iterator for IntoColumns {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for IntoColumns {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

impl ExactSizeIterator for IntoColumns {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl FusedIterator for IntoColumns {}

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

impl DoubleEndedIterator for Values<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

impl ExactSizeIterator for Values<'_> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl FusedIterator for Values<'_> {}

pub struct IntoValues {
    iter: ecow::vec::IntoIter<Value>,
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

impl FusedIterator for IntoValues {}

// pub struct Drain<'a> {
//     iter: std::slice::Iter<'a, (String, Value)>,
// }

// impl Iterator for Drain<'_> {
//     type Item = (String, Value);

//     fn next(&mut self) -> Option<Self::Item> {
//         self.iter
//             .next()
//             .map(|(col, val)| (col.clone(), val.clone()))
//     }

//     fn size_hint(&self) -> (usize, Option<usize>) {
//         self.iter.size_hint()
//     }
// }

// impl DoubleEndedIterator for Drain<'_> {
//     fn next_back(&mut self) -> Option<Self::Item> {
//         self.iter
//             .next_back()
//             .map(|(col, val)| (col.clone(), val.clone()))
//     }
// }

// impl ExactSizeIterator for Drain<'_> {
//     fn len(&self) -> usize {
//         self.iter.len()
//     }
// }

// impl FusedIterator for Drain<'_> {}

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
