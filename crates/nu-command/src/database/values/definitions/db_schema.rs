use super::db_table::DbTable;

#[derive(Clone, PartialEq, Debug)]
pub struct DbSchema {
    pub name: String,
    pub tables: Vec<DbTable>,
}
