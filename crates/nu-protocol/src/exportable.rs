use crate::DeclId;

pub enum Exportable {
    Decl { name: Vec<u8>, id: DeclId },
}
