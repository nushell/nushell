use std::any;
use std::fmt::{Debug, Error, Formatter};
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id<T> {
    inner: usize,
    _phantom: PhantomData<T>,
}

impl<T> Id<T> {
    /// Creates a new `Id`.
    ///
    /// Using a distinct type like `Id` instead of `usize` helps us avoid mixing plain integers
    /// with identifiers.
    #[inline]
    pub const fn new(inner: usize) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
        }
    }

    /// Returns the inner `usize` value.
    ///
    /// This requires an explicit call, ensuring we only use the raw value when intended.
    #[inline]
    pub const fn get(self) -> usize {
        self.inner
    }
}

impl<T> Debug for Id<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let marker = any::type_name::<T>().split("::").last().expect("not empty");
        write!(f, "{marker}Id({})", self.inner)
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

pub mod marker {
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Var;
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Decl;
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Block;
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Module;
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Overlay;
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct File;
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct VirtualPath;
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Span;
}

pub type VarId = Id<marker::Var>;
pub type DeclId = Id<marker::Decl>;
pub type BlockId = Id<marker::Block>;
pub type ModuleId = Id<marker::Module>;
pub type OverlayId = Id<marker::Overlay>;
pub type FileId = Id<marker::File>;
pub type VirtualPathId = Id<marker::VirtualPath>;
pub type SpanId = Id<marker::Span>;

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
