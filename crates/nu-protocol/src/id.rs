use std::any;
use std::cmp::Ordering;
use std::fmt::{Debug, Error, Formatter};
use std::hash::{Hasher, Hash};
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

pub struct Id<T> {
    inner: usize,
    _phantom: PhantomData<T>,
}

impl<T> Id<T> {
    /// Creates a new `Id`.
    ///
    /// Using a distinct type like `Id` instead of `usize` helps us avoid mixing plain integers
    /// with identifiers.
    pub const fn new(inner: usize) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
        }
    }

    /// Returns the inner `usize` value.
    ///
    /// This requires an explicit call, ensuring we only use the raw value when intended.
    pub const fn get(self) -> usize {
        self.inner
    }

    /// Casts the `Id<T>` into `Id<U>` without changing the inner value.
    ///
    /// # Attention
    /// Ensure the type cast is correct. If the wrong type is used, it may indicate a typing mistake elsewhere.
    pub const fn cast<U>(self) -> Id<U> {
        Id {
            inner: self.inner,
            _phantom: PhantomData
        }
    }
}

impl<T> Debug for Id<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let marker = any::type_name::<T>().split("::").last().expect("not empty");
        write!(f, "{marker}Id({})", self.inner)
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _phantom: self._phantom.clone(),
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T> Eq for Id<T> {}

impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Serialize for Id<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for Id<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let inner = usize::deserialize(deserializer)?;
        Ok(Self {
            inner,
            _phantom: PhantomData,
        })
    }
}

impl<T> Hash for Id<T> {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.inner.hash(state)
    }
}

pub mod marker {
    pub struct Var;
    pub struct Decl;
    pub struct Block;
    pub struct Module;
    pub struct Overlay;
    pub struct File;
    pub struct VirtualPath;
}

pub type VarId = Id<marker::Var>;
pub type DeclId = Id<marker::Decl>;
pub type BlockId = Id<marker::Block>;
pub type ModuleId = Id<marker::Module>;
pub type OverlayId = Id<marker::Overlay>;
pub type FileId = Id<marker::File>;
pub type VirtualPathId = Id<marker::VirtualPath>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SpanId(pub usize); // more robust ID style used in the new parser

/// An ID for an [IR](crate::ir) register. `%n` is a common shorthand for `RegId(n)`.
///
/// Note: `%0` is allocated with the block input at the beginning of a compiled block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
pub struct RegId(pub u32);

impl std::fmt::Display for RegId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "%{}", self.0)
    }
}
