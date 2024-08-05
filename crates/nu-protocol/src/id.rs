use serde::{Deserialize, Serialize};

pub type VarId = usize;
pub type DeclId = usize;
pub type BlockId = usize;
pub type ModuleId = usize;
pub type OverlayId = usize;
pub type FileId = usize;
pub type VirtualPathId = usize;

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
