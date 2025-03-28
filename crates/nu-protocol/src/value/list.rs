use crate::{FromValue, IntoValue, ShellError, Span, Value};
use serde::{
    de::{SeqAccess, Visitor},
    ser::SerializeSeq,
    Deserialize, Deserializer, Serialize,
};
use std::{
    borrow::Borrow, collections::VecDeque, error::Error, fmt, iter::FusedIterator, ops::Deref,
};

/// The error returned when attempting to index a [`List`] out of bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexOutOfBoundsError {
    /// The index that was out of bounds.
    pub index: usize,
    /// The length of the [`List`].
    pub length: usize,
}

impl fmt::Display for IndexOutOfBoundsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { index, length } = *self;
        write!(
            f,
            "index {index} was out of bounds for list with length {length}"
        )
    }
}

impl Error for IndexOutOfBoundsError {}

/// A list of [`Value`]s.
///
/// A [`List`] is a contiguous growable array similar to a [`Vec`]. To create a [`List`] use either:
/// - [`List::new`]
/// - [`List::with_capacity`]
/// - the [`list!`] macro
/// - `collect` on an iterator, or [`List::from_iter`]
///
/// Additionally, you can convert [`Vec`]s and arrays into [`List`]s using the [`From`] trait.
#[derive(Clone, PartialEq)]
pub struct List(Vec<Value>);

impl List {
    /// Constructs a new, empty [`List`].
    #[inline]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Constructs a new, empty [`List`] with at least the specified capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    /// Constructs a new [`List`] containing `n` copies of `value`.
    #[inline]
    pub fn from_elem(value: Value, n: usize) -> Self {
        Self(vec![value; n])
    }

    /// Returns whether the [`List`] contains no values.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of values in the [`List`].
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns the total number of values the vector can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Returns a slice containing all the values in a [`List`].
    ///
    /// Equivalent to `&list[..]`.
    #[inline]
    pub fn as_slice(&self) -> &[Value] {
        self.0.as_slice()
    }

    #[inline]
    pub fn make_mut(&mut self) -> &mut [Value] {
        &mut self.0
    }

    /// Clears a [`List`], removing all values.
    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Appends a value to the end of a [`List`].
    #[inline]
    pub fn push(&mut self, value: Value) {
        self.0.push(value);
    }

    /// Removes the last value from a [`List`] and returns it, or `None` if the list is empty.
    #[inline]
    pub fn pop(&mut self) -> Option<Value> {
        self.0.pop()
    }

    /// Inserts `value` at position `index` within the [`List`], shifting all values after it to the right.
    ///
    /// # Errors
    ///
    /// If `index` is greater than the length of the list, then a [`IndexOutOfBoundsError`] is returned.
    #[inline]
    pub fn insert(&mut self, index: usize, value: Value) -> Result<(), IndexOutOfBoundsError> {
        let length = self.len();
        if index <= length {
            self.0.insert(index, value);
            Ok(())
        } else {
            Err(IndexOutOfBoundsError { index, length })
        }
    }

    #[inline]
    pub fn remove(&mut self, index: usize) -> Result<Value, IndexOutOfBoundsError> {
        let length = self.len();
        if index < length {
            Ok(self.0.remove(index))
        } else {
            Err(IndexOutOfBoundsError { index, length })
        }
    }

    #[inline]
    pub fn retain_mut(&mut self, f: impl FnMut(&mut Value) -> bool) {
        self.0.retain_mut(f);
    }

    #[inline]
    pub fn retain(&mut self, f: impl FnMut(&Value) -> bool) {
        self.0.retain(f);
    }

    #[inline]
    pub fn truncate(&mut self, target: usize) {
        self.0.truncate(target);
    }

    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    #[inline]
    pub fn extend_from_slice(&mut self, slice: &[Value]) {
        self.0.extend_from_slice(slice);
    }

    // Other methods we may want, but `ecow::EcoVec` does not have them yet
    // splice
    // drain
    // extend_from_within
    // reserve_exact
    // resize
    // resize_with
}

impl Default for List {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for List {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&self.0).finish()
    }
}

impl Deref for List {
    type Target = [Value];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<[Value]> for List {
    #[inline]
    fn as_ref(&self) -> &[Value] {
        self
    }
}

impl Borrow<[Value]> for List {
    #[inline]
    fn borrow(&self) -> &[Value] {
        self
    }
}

impl FromIterator<Value> for List {
    #[inline]
    fn from_iter<T: IntoIterator<Item = Value>>(iter: T) -> Self {
        Self(Vec::from_iter(iter))
    }
}

impl Extend<Value> for List {
    #[inline]
    fn extend<T: IntoIterator<Item = Value>>(&mut self, iter: T) {
        self.0.extend(iter)
    }
}

pub struct IntoIter {
    iter: std::vec::IntoIter<Value>,
}

impl Iterator for IntoIter {
    type Item = Value;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for IntoIter {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

impl ExactSizeIterator for IntoIter {
    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl FusedIterator for IntoIter {}

impl IntoIterator for List {
    type Item = Value;

    type IntoIter = IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            iter: self.0.into_iter(),
        }
    }
}

pub struct Iter<'a> {
    iter: std::slice::Iter<'a, Value>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Value;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for Iter<'_> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

impl ExactSizeIterator for Iter<'_> {
    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl FusedIterator for Iter<'_> {}

impl<'a> IntoIterator for &'a List {
    type Item = &'a Value;

    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Iter {
            iter: self.0.iter(),
        }
    }
}

impl<'a> From<&'a [Value]> for List {
    #[inline]
    fn from(slice: &'a [Value]) -> Self {
        Self(slice.into())
    }
}

impl<'a, const N: usize> From<&'a [Value; N]> for List {
    #[inline]
    fn from(slice: &'a [Value; N]) -> Self {
        Self(slice.into())
    }
}

impl<'a> From<&'a mut [Value]> for List {
    #[inline]
    fn from(slice: &'a mut [Value]) -> Self {
        Self(slice.into())
    }
}

impl<'a, const N: usize> From<&'a mut [Value; N]> for List {
    #[inline]
    fn from(slice: &'a mut [Value; N]) -> Self {
        Self(slice.into())
    }
}

impl From<Vec<Value>> for List {
    #[inline]
    fn from(vec: Vec<Value>) -> Self {
        Self(vec)
    }
}

impl From<List> for Vec<Value> {
    #[inline]
    fn from(list: List) -> Self {
        list.0
    }
}

impl From<VecDeque<Value>> for List {
    #[inline]
    fn from(deque: VecDeque<Value>) -> Self {
        Self(deque.into())
    }
}

impl From<List> for VecDeque<Value> {
    #[inline]
    fn from(list: List) -> Self {
        list.0.into()
    }
}

impl<const N: usize> From<[Value; N]> for List {
    #[inline]
    fn from(array: [Value; N]) -> Self {
        Self(array.into())
    }
}

impl<const N: usize> TryFrom<List> for [Value; N] {
    type Error = List;

    #[inline]
    fn try_from(value: List) -> Result<Self, Self::Error> {
        value.0.try_into().map_err(List)
    }
}

impl FromValue for List {
    #[inline]
    fn from_value(v: Value) -> Result<Self, ShellError> {
        v.into_list()
    }
}

impl IntoValue for List {
    #[inline]
    fn into_value(self, span: Span) -> Value {
        Value::list(self, span)
    }
}

impl<T, const N: usize> PartialEq<&[T; N]> for List
where
    Value: PartialEq<T>,
{
    #[inline]
    fn eq(&self, other: &&[T; N]) -> bool {
        self.0 == *other
    }
}

impl<T> PartialEq<&[T]> for List
where
    Value: PartialEq<T>,
{
    #[inline]
    fn eq(&self, other: &&[T]) -> bool {
        self.0 == *other
    }
}

impl<T> PartialEq<&mut [T]> for List
where
    Value: PartialEq<T>,
{
    #[inline]
    fn eq(&self, other: &&mut [T]) -> bool {
        self.0 == *other
    }
}

impl<T, const N: usize> PartialEq<[T; N]> for List
where
    Value: PartialEq<T>,
{
    #[inline]
    fn eq(&self, other: &[T; N]) -> bool {
        self.0 == other
    }
}

impl<T> PartialEq<[T]> for List
where
    Value: PartialEq<T>,
{
    #[inline]
    fn eq(&self, other: &[T]) -> bool {
        self.0 == other
    }
}

impl<T> PartialEq<Vec<T>> for List
where
    Value: PartialEq<T>,
{
    #[inline]
    fn eq(&self, other: &Vec<T>) -> bool {
        self.0 == *other
    }
}

impl<T, const N: usize> PartialEq<List> for &[T; N]
where
    T: PartialEq<Value>,
{
    #[inline]
    fn eq(&self, other: &List) -> bool {
        *self == other.as_slice()
    }
}

impl<T> PartialEq<List> for &[T]
where
    T: PartialEq<Value>,
{
    #[inline]
    fn eq(&self, other: &List) -> bool {
        *self == other.as_slice()
    }
}

impl<T> PartialEq<List> for &mut [T]
where
    T: PartialEq<Value>,
{
    #[inline]
    fn eq(&self, other: &List) -> bool {
        *self == other.as_slice()
    }
}

impl<T, const N: usize> PartialEq<List> for [T; N]
where
    T: PartialEq<Value>,
{
    #[inline]
    fn eq(&self, other: &List) -> bool {
        self == other.as_slice()
    }
}

impl<T> PartialEq<List> for [T]
where
    T: PartialEq<Value>,
{
    #[inline]
    fn eq(&self, other: &List) -> bool {
        self == other.as_slice()
    }
}

impl<T> PartialEq<List> for Vec<T>
where
    T: PartialEq<Value>,
{
    #[inline]
    fn eq(&self, other: &List) -> bool {
        self == other.as_slice()
    }
}

impl Serialize for List {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for item in self {
            seq.serialize_element(item)?;
        }
        seq.end()
    }
}

struct ListVisitor;

impl<'a> Visitor<'a> for ListVisitor {
    type Value = List;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a sequence")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'a>,
    {
        let len = seq.size_hint().unwrap_or(0);
        let mut values = List::with_capacity(len);
        while let Some(value) = seq.next_element()? {
            values.push(value)
        }
        Ok(values)
    }
}

impl<'de> Deserialize<'de> for List {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(ListVisitor)
    }
}

#[doc(hidden)]
#[inline]
pub fn from_vec(vec: Vec<Value>) -> List {
    List(vec)
}

/// Creates a [`List`] containing the arguments.
///
/// `list!` allows [`Lists`]s to be defined with the same syntax as array expressions.
/// There are two forms of this macro:
///
/// - Create a [`List`] containing a given list of values:
///
/// ```
/// # use nu_protocol::{list, Span, Value};
/// let span = Span::test_data();
/// let list = list![
///     Value::int(1, span),
///     Value::int(2, span),
///     Value::int(3, span),
/// ];
/// assert_eq!(list.get(0), Some(&Value::int(1, span)));
/// assert_eq!(list.get(1), Some(&Value::int(2, span)));
/// assert_eq!(list.get(2), Some(&Value::int(3, span)));
/// assert_eq!(list.get(3), None);
/// ```
///
/// - Create a [`List`] from a given value and length:
///
/// ```
/// # use nu_protocol::{list, Span, Value};
/// let span = Span::test_data();
/// let list = list![Value::int(1, span); 3];
/// assert_eq!(
///     list,
///     [
///         Value::int(1, span),
///         Value::int(1, span),
///         Value::int(1, span),
///     ],
/// );
/// ```
///
/// Note that unlike array expressions the number of elements doesn't have to be a constant.
///
/// Also, note that `list![expr; 0]` is allowed, and produces an empty list.
/// This will still evaluate `expr`, however, and immediately drop the resulting value.
#[macro_export]
macro_rules! list {
    () => (
        $crate::List::new()
    );
    ($elem:expr; $n:expr) => (
        $crate::List::from_elem($elem, $n)
    );
    ($($x:expr),+ $(,)?) => (
        $crate::list::from_vec(vec![$($x,)+])
    );
}
