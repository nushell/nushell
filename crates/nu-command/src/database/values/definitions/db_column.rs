use crate::database::values::definitions::db_row::DbRow;

#[derive(Debug)]
pub struct DbColumn {
    /// Column Index
    pub cid: Option<i32>,
    /// Column Name
    pub name: Option<String>,
    /// Column Type
    pub r#type: Option<String>,
    /// Column has a NOT NULL constraint
    pub notnull: Option<i16>,
    /// Column DEFAULT Value
    pub default: Option<String>,
    /// Column is part of the PRIMARY KEY
    pub pk: Option<i16>,
}

impl DbRow for DbColumn {
    fn fields(&self) -> Vec<String> {
        vec![
            "cid".to_string(),
            "name".to_string(),
            "type".to_string(),
            "notnull".to_string(),
            "default".to_string(),
            "pk".to_string(),
        ]
    }

    fn columns(&self) -> Vec<String> {
        vec![
            self.cid
                .as_ref()
                .map_or(String::new(), |cid| cid.to_string()),
            self.name
                .as_ref()
                .map_or(String::new(), |name| name.to_string()),
            self.r#type
                .as_ref()
                .map_or(String::new(), |r#type| r#type.to_string()),
            self.notnull
                .as_ref()
                .map_or(String::new(), |notnull| notnull.to_string()),
            self.default
                .as_ref()
                .map_or(String::new(), |default| default.to_string()),
            self.pk.as_ref().map_or(String::new(), |pk| pk.to_string()),
        ]
    }
}
