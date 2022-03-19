use crate::{AliasId, BlockId, DeclId};

pub enum Exportable {
    Decl(DeclId),
    Alias(AliasId),
    EnvVar(BlockId),
}
