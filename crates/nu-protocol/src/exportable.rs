use crate::{DeclId, ModuleId};

pub enum Exportable {
    Decl { name: Vec<u8>, id: DeclId },
    Module { name: Vec<u8>, id: ModuleId },
}
