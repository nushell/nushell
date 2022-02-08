use std::collections::HashMap;

use crate::{BlockId, Value, VarId};

#[derive(Clone, Debug)]
pub struct CaptureBlock {
    pub block_id: BlockId,
    pub captures: HashMap<VarId, Value>,
}
