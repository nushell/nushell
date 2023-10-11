use super::db_table::DbTable;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DbSchema {
    pub name: String,
    pub tables: Vec<DbTable>,
}
