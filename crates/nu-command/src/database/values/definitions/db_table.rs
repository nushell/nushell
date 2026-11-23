#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DbTable {
    pub name: String,
    pub engine: Option<String>,
    pub schema: Option<String>,
}
