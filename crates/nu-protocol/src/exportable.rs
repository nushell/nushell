use crate::{AliasId, DeclId};

pub enum Exportable {
    Decl { name: Vec<u8>, id: DeclId },
    Alias { name: Vec<u8>, id: AliasId },
}
