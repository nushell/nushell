use crate::{BlockId, Signature};

#[derive(Clone, Debug)]
pub struct Declaration {
    pub signature: Signature,
    pub body: Option<BlockId>,
}
