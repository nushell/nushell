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
