use serde::{Deserialize, Serialize};
use std::{fmt, ops, sync::Arc};

/// A container that transparently shares a value when possible, but clones on mutate.
///
/// Unlike `Arc`, this is only intended to help save memory usage and reduce the amount of effort
/// required to clone unmodified values with easy to use copy-on-write.
///
/// This should more or less reflect the API of [`std::borrow::Cow`] as much as is sensible.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct SharedCow<T: Clone>(Arc<T>);

impl<T: Clone> SharedCow<T> {
    /// Create a new `Shared` value.
    pub fn new(value: T) -> SharedCow<T> {
        SharedCow(Arc::new(value))
    }

    /// Take an exclusive clone of the shared value, or move and take ownership if it wasn't shared.
    pub fn into_owned(self: SharedCow<T>) -> T {
        // Optimized: if the Arc is not shared, just unwraps the Arc
        match Arc::try_unwrap(self.0) {
            Ok(value) => value,
            Err(arc) => (*arc).clone(),
        }
    }

    /// Get a mutable reference to the value inside the [`SharedCow`]. This will result in a clone
    /// being created only if the value was shared with multiple references.
    pub fn to_mut(&mut self) -> &mut T {
        Arc::make_mut(&mut self.0)
    }

    /// Convert the `Shared` value into an `Arc`
    pub fn into_arc(value: SharedCow<T>) -> Arc<T> {
        value.0
    }

    /// Return the number of references to the shared value.
    pub fn ref_count(value: &SharedCow<T>) -> usize {
        Arc::strong_count(&value.0)
    }
}

impl<T> From<T> for SharedCow<T>
where
    T: Clone,
{
    fn from(value: T) -> Self {
        SharedCow::new(value)
    }
}

impl<T> From<Arc<T>> for SharedCow<T>
where
    T: Clone,
{
    fn from(value: Arc<T>) -> Self {
        SharedCow(value)
    }
}

impl<T> fmt::Debug for SharedCow<T>
where
    T: fmt::Debug + Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Appears transparent
        (*self.0).fmt(f)
    }
}

impl<T> fmt::Display for SharedCow<T>
where
    T: fmt::Display + Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (*self.0).fmt(f)
    }
}

impl<T: Clone> Serialize for SharedCow<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, T: Clone> Deserialize<'de> for SharedCow<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Arc::new).map(SharedCow)
    }
}

impl<T: Clone> ops::Deref for SharedCow<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
