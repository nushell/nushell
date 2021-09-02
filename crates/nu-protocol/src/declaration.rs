use crate::{BlockId, Signature};

pub struct Declaration {
    pub signature: Box<Signature>,
    pub body: Option<BlockId>,
}
