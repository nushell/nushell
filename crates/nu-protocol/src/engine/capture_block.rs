use std::collections::HashMap;

use crate::{BlockId, Value, VarId};

#[derive(Clone, Debug)]
pub struct Closure {
    pub block_id: BlockId,
    pub captures: HashMap<VarId, Value>,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub block_id: BlockId,
}
