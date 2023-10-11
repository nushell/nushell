use super::db_row::DbRow;

#[derive(Debug)]
pub struct DbIndex {
    pub name: Option<String>,
    pub column_name: Option<String>,
    pub seqno: Option<i16>,
}

impl DbRow for DbIndex {
    fn fields(&self) -> Vec<String> {
        vec![
            "name".to_string(),
            "column_name".to_string(),
            "seqno".to_string(),
        ]
    }

    fn columns(&self) -> Vec<String> {
        vec![
            self.name
                .as_ref()
                .map_or(String::new(), |name| name.to_string()),
            self.column_name
                .as_ref()
                .map_or(String::new(), |column_name| column_name.to_string()),
            self.seqno
                .as_ref()
                .map_or(String::new(), |seqno| seqno.to_string()),
        ]
    }
}
