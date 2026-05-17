use std::any;
use std::fmt::{Debug, Display, Error, Formatter};
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id<M, V = usize> {
    inner: V,
    _phantom: PhantomData<M>,
}

impl<M, V> Id<M, V> {
    /// Creates a new `Id`.
    ///
    /// Using a distinct type like `Id` instead of `usize` helps us avoid mixing plain integers
    /// with identifiers.
    #[inline]
    pub const fn new(inner: V) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
        }
    }
}

impl<M, V> Id<M, V>
where
    V: Copy,
{
    /// Returns the inner value.
    ///
    /// This requires an explicit call, ensuring we only use the raw value when intended.
    #[inline]
    pub const fn get(self) -> V {
        self.inner
    }
}

impl<M> Id<M, usize> {
    pub const ZERO: Self = Self::new(0);
}

impl<M, V> Debug for Id<M, V>
where
    V: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let marker = any::type_name::<M>().split("::").last().expect("not empty");
        write!(f, "{marker}Id({})", self.inner)
    }
}

impl<M, V> Serialize for Id<M, V>
where
    V: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'de, M, V> Deserialize<'de> for Id<M, V>
where
    V: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let inner = V::deserialize(deserializer)?;
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
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Reg;
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Job;
}

pub type VarId = Id<marker::Var>;
pub type DeclId = Id<marker::Decl>;
pub type BlockId = Id<marker::Block>;
pub type ModuleId = Id<marker::Module>;
pub type OverlayId = Id<marker::Overlay>;
pub type FileId = Id<marker::File>;
pub type VirtualPathId = Id<marker::VirtualPath>;
pub type SpanId = Id<marker::Span>;
pub type JobId = Id<marker::Job>;

/// An ID for an [IR](crate::ir) register.
///
/// `%n` is a common shorthand for `RegId(n)`.
///
/// Note: `%0` is allocated with the block input at the beginning of a compiled block.
pub type RegId = Id<marker::Reg, u32>;

impl Display for JobId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl Display for RegId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "%{}", self.get())
    }
}
