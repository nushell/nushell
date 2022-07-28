use crate::{AliasId, BlockId, DeclId};

pub enum Exportable {
    Decl { name: Vec<u8>, id: DeclId },
    Alias { name: Vec<u8>, id: AliasId },
    EnvVar { name: Vec<u8>, id: BlockId },
}
