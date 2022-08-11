#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DbTable {
    pub name: String,
    pub create_time: Option<chrono::DateTime<chrono::Utc>>,
    pub update_time: Option<chrono::DateTime<chrono::Utc>>,
    pub engine: Option<String>,
    pub schema: Option<String>,
}
