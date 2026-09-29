use nu_protocol::{DeclId, ModuleId, TypeDef, VarId};
use std::sync::Arc;

/// Symbol that can be exported with its associated name and ID
pub enum Exportable {
    Decl {
        name: Vec<u8>,
        id: DeclId,
    },
    Module {
        name: Vec<u8>,
        id: ModuleId,
    },
    VarDecl {
        name: Vec<u8>,
        id: VarId,
    },
    /// A named type declared with `export type`.
    Type {
        name: Vec<u8>,
        def: Arc<TypeDef>,
    },
}
