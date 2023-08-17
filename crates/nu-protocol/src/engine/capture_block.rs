use std::collections::HashMap;

use crate::{BlockId, SpannedValue, VarId};

#[derive(Clone, Debug)]
pub struct Closure {
    pub block_id: BlockId,
    pub captures: HashMap<VarId, SpannedValue>,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub block_id: BlockId,
}
