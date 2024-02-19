use nu_protocol::{DeclId, ModuleId, VarId};

pub enum Exportable {
    Decl { name: Vec<u8>, id: DeclId },
    Module { name: Vec<u8>, id: ModuleId },
    VarDecl { name: Vec<u8>, id: VarId },
}
