use nu_protocol::{DeclId, ModuleId, TypeAliasId, VarId};

/// Symbol that can be exported with its associated name and ID
pub enum Exportable {
    Decl { name: Vec<u8>, id: DeclId },
    Module { name: Vec<u8>, id: ModuleId },
    VarDecl { name: Vec<u8>, id: VarId },
    TypeAlias { name: Vec<u8>, id: TypeAliasId },
}
