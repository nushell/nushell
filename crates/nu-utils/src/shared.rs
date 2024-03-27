use serde::{Deserialize, Serialize};
use std::{fmt, ops, sync::Arc};

/// A container that transparently shares a value when possible, but clones on mutate.
///
/// Unlike `Arc`, this is only intended to help save memory usage and reduce the amount of effort
/// required to clone unmodified values with easy to use copy-on-write.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct Shared<T: Clone>(Arc<T>);

impl<T: Clone> Shared<T> {
    /// Create a new `Shared` value.
    pub fn new(value: T) -> Shared<T> {
        Shared(Arc::new(value))
    }

    /// Take an exclusive clone of the shared value, or move and take ownership if it wasn't shared.
    pub fn unwrap(value: Shared<T>) -> T {
        // Optimized: if the Arc is not shared, just unwraps the Arc
        match Arc::try_unwrap(value.0) {
            Ok(value) => value,
            Err(arc) => (*arc).clone(),
        }
    }

    /// Convert the `Shared` value into an `Arc`
    pub fn into_arc(value: Shared<T>) -> Arc<T> {
        value.0
    }

    /// Return the number of references to the shared value.
    pub fn ref_count(value: &Shared<T>) -> usize {
        Arc::strong_count(&value.0)
    }
}

impl<T> From<T> for Shared<T>
where
    T: Clone,
{
    fn from(value: T) -> Self {
        Shared::new(value)
    }
}

impl<T> From<Arc<T>> for Shared<T>
where
    T: Clone,
{
    fn from(value: Arc<T>) -> Self {
        Shared(value)
    }
}

impl<T> fmt::Debug for Shared<T>
where
    T: fmt::Debug + Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Appears transparent
        (*self.0).fmt(f)
    }
}

impl<T> fmt::Display for Shared<T>
where
    T: fmt::Display + Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (*self.0).fmt(f)
    }
}

impl<T: Clone> Serialize for Shared<T>
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

impl<'de, T: Clone> Deserialize<'de> for Shared<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Arc::new).map(Shared)
    }
}

impl<T: Clone> ops::Deref for Shared<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Clone> ops::DerefMut for Shared<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.0)
    }
}

impl<T> IntoIterator for Shared<T>
where
    T: Clone + IntoIterator,
{
    type Item = <T as IntoIterator>::Item;
    type IntoIter = <T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (*self).clone().into_iter()
    }
}
