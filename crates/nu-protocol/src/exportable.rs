use crate::{BlockId, DeclId};

pub enum Exportable {
    Decl(DeclId),
    EnvVar(BlockId),
}
