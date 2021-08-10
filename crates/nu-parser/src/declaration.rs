use crate::{BlockId, Signature};

#[derive(Clone, Debug)]
pub struct Declaration {
    pub signature: Box<Signature>,
    pub body: Option<BlockId>,
}
