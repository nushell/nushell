use super::db_table::DbTable;

// Thank you gobang
// https://github.com/TaKO8Ki/gobang/blob/main/database-tree/src/lib.rs

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Db {
    pub name: String,
    pub tables: Vec<DbTable>,
}

impl Db {
    pub fn new(database: String, tables: Vec<DbTable>) -> Self {
        Self {
            name: database,
            tables,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn tables(&self) -> Vec<DbTable> {
        self.tables.clone()
    }
}
