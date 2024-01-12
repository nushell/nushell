use crate::{BlockId, Value, VarId};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Closure {
    pub block_id: BlockId,
    pub captures: Vec<(VarId, Value)>,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub block_id: BlockId,
}
