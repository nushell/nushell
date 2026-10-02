use std::any;
use std::fmt::{Debug, Display, Error, Formatter};
use std::marker::PhantomData;

use crate::relocation::{self, IdMarker};
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

// Ids that index engine-state tables go through the relocation hooks, so a serialized parse
// delta can be loaded into a different engine state (see `crate::relocation`). Outside of such a
// (de)serialization the hooks leave the value unchanged.
impl<M: IdMarker> Serialize for Id<M, usize> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let Some(kind) = M::KIND {
            relocation::note_id(kind, self.inner);
        }
        self.inner.serialize(serializer)
    }
}

impl<'de, M: IdMarker> Deserialize<'de> for Id<M, usize> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let inner = usize::deserialize(deserializer)?;
        let inner = match M::KIND {
            Some(kind) => relocation::map_id(kind, inner).map_err(serde::de::Error::custom)?,
            None => inner,
        };
        Ok(Self::new(inner))
    }
}

impl<M> Serialize for Id<M, u32> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'de, M> Deserialize<'de> for Id<M, u32> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        u32::deserialize(deserializer).map(Self::new)
    }
}

pub mod marker {
    use crate::relocation::{IdKind, IdMarker};

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

    impl IdMarker for Var {
        const KIND: Option<IdKind> = Some(IdKind::Var);
    }
    impl IdMarker for Decl {
        const KIND: Option<IdKind> = Some(IdKind::Decl);
    }
    impl IdMarker for Block {
        const KIND: Option<IdKind> = Some(IdKind::Block);
    }
    impl IdMarker for Module {
        const KIND: Option<IdKind> = Some(IdKind::Module);
    }
    impl IdMarker for File {
        const KIND: Option<IdKind> = Some(IdKind::File);
    }
    impl IdMarker for Span {
        const KIND: Option<IdKind> = Some(IdKind::Span);
    }
    // Overlay ids index the overlays of one scope frame, so they stay as they are.
    impl IdMarker for Overlay {}
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
