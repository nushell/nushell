use super::db_row::DbRow;

#[derive(Debug)]
pub struct DbConstraint {
    pub name: String,
    pub column_name: String,
    pub origin: String,
}

impl DbRow for DbConstraint {
    fn fields(&self) -> Vec<String> {
        vec![
            "name".to_string(),
            "column_name".to_string(),
            "origin".to_string(),
        ]
    }

    fn columns(&self) -> Vec<String> {
        vec![
            self.name.to_string(),
            self.column_name.to_string(),
            self.origin.to_string(),
        ]
    }
}
