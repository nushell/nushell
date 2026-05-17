use super::db_row::DbRow;

#[derive(Debug)]
pub struct DbForeignKey {
    pub column_name: Option<String>,
    pub ref_table: Option<String>,
    pub ref_column: Option<String>,
}

impl DbRow for DbForeignKey {
    fn fields(&self) -> Vec<String> {
        vec![
            "column_name".to_string(),
            "ref_table".to_string(),
            "ref_column".to_string(),
        ]
    }

    fn columns(&self) -> Vec<String> {
        vec![
            self.column_name
                .as_ref()
                .map_or(String::new(), |r#type| r#type.to_string()),
            self.ref_table
                .as_ref()
                .map_or(String::new(), |r#type| r#type.to_string()),
            self.ref_column
                .as_ref()
                .map_or(String::new(), |r#type| r#type.to_string()),
        ]
    }
}
