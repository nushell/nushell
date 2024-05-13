use crate::database::{values_to_sql, SQLiteDatabase, MEMORY_DB};
use nu_engine::command_prelude::*;
use rusqlite::params_from_iter;

#[derive(Clone)]
pub struct StorInsert;

impl Command for StorInsert {
    fn name(&self) -> &str {
        "stor insert"
    }

    fn signature(&self) -> Signature {
        Signature::build("stor insert")
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .required_named(
                "table-name",
                SyntaxShape::String,
                "name of the table you want to insert into",
                Some('t'),
            )
            .required_named(
                "data-record",
                SyntaxShape::Record(vec![]),
                "a record of column names and column values to insert into the specified table",
                Some('d'),
            )
            .allow_variants_without_examples(true)
            .category(Category::Database)
    }

    fn usage(&self) -> &str {
        "Insert information into a specified table in the in-memory sqlite database."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["sqlite", "storing", "table", "saving"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Insert data the in-memory sqlite database using a data-record of column-name and column-value pairs",
            example: "stor insert --table-name nudb --data-record {bool1: true, int1: 5, float1: 1.1, str1: fdncred, datetime1: 2023-04-17}",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let table_name: Option<String> = call.get_flag(engine_state, stack, "table-name")?;
        let columns: Option<Record> = call.get_flag(engine_state, stack, "data-record")?;
        // let config = engine_state.get_config();
        let db = Box::new(SQLiteDatabase::new(std::path::Path::new(MEMORY_DB), None));

        process(table_name, span, &db, columns)?;

        Ok(Value::custom(db, span).into_pipeline_data())
    }
}

fn process(
    table_name: Option<String>,
    span: Span,
    db: &SQLiteDatabase,
    columns: Option<Record>,
) -> Result<(), ShellError> {
    if table_name.is_none() {
        return Err(ShellError::MissingParameter {
            param_name: "requires at table name".into(),
            span,
        });
    }
    let new_table_name = table_name.unwrap_or("table".into());
    if let Ok(conn) = db.open_connection() {
        match columns {
            Some(record) => {
                let mut create_stmt = format!("INSERT INTO {} ( ", new_table_name);
                let cols = record.columns();
                cols.for_each(|col| {
                    create_stmt.push_str(&format!("{}, ", col));
                });
                if create_stmt.ends_with(", ") {
                    create_stmt.pop();
                    create_stmt.pop();
                }

                // Values are set as placeholders.
                create_stmt.push_str(") VALUES ( ");
                for (index, _) in record.columns().enumerate() {
                    create_stmt.push_str(&format!("?{}, ", index + 1));
                }

                if create_stmt.ends_with(", ") {
                    create_stmt.pop();
                    create_stmt.pop();
                }

                create_stmt.push(')');

                // dbg!(&create_stmt);

                // Get the params from the passed values
                let params = values_to_sql(record.values().cloned())?;

                conn.execute(&create_stmt, params_from_iter(params))
                    .map_err(|err| ShellError::GenericError {
                        error: "Failed to open SQLite connection in memory from insert".into(),
                        msg: err.to_string(),
                        span: Some(Span::test_data()),
                        help: None,
                        inner: vec![],
                    })?;
            }
            None => {
                return Err(ShellError::MissingParameter {
                    param_name: "requires at least one column".into(),
                    span,
                });
            }
        };
    }
    // dbg!(db.clone());
    Ok(())
}

#[cfg(test)]
mod test {
    use chrono::DateTime;

    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(StorInsert {})
    }

    #[test]
    fn test_process_with_simple_parameters() {
        let db = Box::new(SQLiteDatabase::new(std::path::Path::new(MEMORY_DB), None));
        let create_stmt = "CREATE TABLE test_process_with_simple_parameters (
            int_column INTEGER,
            real_column REAL,
            str_column VARCHAR(255),
            bool_column BOOLEAN,
            date_column DATETIME DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW'))
        )";

        let conn = db.open_connection().expect("Test was unable to open connection.");
        conn.execute(&create_stmt, []).expect("Failed to create table as part of test.");
        let table_name = Some("test_process_with_simple_parameters".to_string());
        let span = Span::unknown();
        let mut columns = Record::new();
        columns.insert(
            "int_column".to_string(),
            Value::test_int(42),
        );
        columns.insert(
            "real_column".to_string(),
            Value::test_float(3.14),
        );
        columns.insert(
            "str_column".to_string(),
            Value::test_string("SimpleString".to_string()),
        );
        columns.insert(
            "bool_column".to_string(),
            Value::test_bool(true),
        );
        columns.insert(
            "date_column".to_string(),
            Value::test_date(DateTime::parse_from_str(
                "2021-12-30 00:00:00 +0000",
                "%Y-%m-%d %H:%M:%S %z",
            ).expect("Date string should parse.")),
        );

        let result = process(table_name, span, &db, Some(columns));

        assert!(result.is_ok());
    }

    #[test]
    fn test_process_string_with_space() {
        let db = Box::new(SQLiteDatabase::new(std::path::Path::new(MEMORY_DB), None));
        let create_stmt = "CREATE TABLE test_process_string_with_space (
            str_column VARCHAR(255)
        )";

        let conn = db.open_connection().expect("Test was unable to open connection.");
        conn.execute(&create_stmt, []).expect("Failed to create table as part of test.");
        let table_name = Some("test_process_string_with_space".to_string());
        let span = Span::unknown();
        let mut columns = Record::new();
        columns.insert(
            "str_column".to_string(),
            Value::test_string("String With Spaces".to_string()),
        );

        let result = process(table_name, span, &db, Some(columns));

        assert!(result.is_ok());
    }

    #[ignore]
    #[test]
    fn test_errors_when_string_too_long() {
        let db = Box::new(SQLiteDatabase::new(std::path::Path::new(MEMORY_DB), None));
        let create_stmt = "CREATE TABLE test_errors_when_string_too_long (
            str_column VARCHAR(8)
        )";

        let conn = db.open_connection().expect("Test was unable to open connection.");
        conn.execute(&create_stmt, []).expect("Failed to create table as part of test.");
        let table_name = Some("test_errors_when_string_too_long".to_string());
        let span = Span::unknown();
        let mut columns = Record::new();
        columns.insert(
            "str_column".to_string(),
            Value::test_string("ThisIsALongString".to_string()),
        );

        let result = process(table_name, span, &db, Some(columns));

        assert!(result.is_err());
    }

    #[ignore]
    #[test]
    fn test_errors_when_param_is_wrong_type() {
        let db = Box::new(SQLiteDatabase::new(std::path::Path::new(MEMORY_DB), None));
        let create_stmt = "CREATE TABLE test_errors_when_param_is_wrong_type (
            int_column INT
        )";

        let conn = db.open_connection().expect("Test was unable to open connection.");
        conn.execute(&create_stmt, []).expect("Failed to create table as part of test.");
        let table_name = Some("test_errors_when_param_is_wrong_type".to_string());
        let span = Span::unknown();
        let mut columns = Record::new();
        columns.insert(
            "int_column".to_string(),
            Value::test_string("ThisIsALongString".to_string()),
        );

        let result = process(table_name, span, &db, Some(columns));

        assert!(result.is_err());
    }

    #[test]
    fn test_errors_when_column_doesnt_exist() {
        let db = Box::new(SQLiteDatabase::new(std::path::Path::new(MEMORY_DB), None));
        let create_stmt = "CREATE TABLE test_errors_when_column_doesnt_exist (
            int_column INT
        )";

        let conn = db.open_connection().expect("Test was unable to open connection.");
        conn.execute(&create_stmt, []).expect("Failed to create table as part of test.");
        let table_name = Some("test_errors_when_column_doesnt_exist".to_string());
        let span = Span::unknown();
        let mut columns = Record::new();
        columns.insert(
            "not_a_column".to_string(),
            Value::test_string("ThisIsALongString".to_string()),
        );

        let result = process(table_name, span, &db, Some(columns));

        assert!(result.is_err());
    }

    #[test]
    fn test_errors_when_table_doesnt_exist() {
        let db = Box::new(SQLiteDatabase::new(std::path::Path::new(MEMORY_DB), None));

        let table_name = Some("test_errors_when_table_doesnt_exist".to_string());
        let span = Span::unknown();
        let mut columns = Record::new();
        columns.insert(
            "str_column".to_string(),
            Value::test_string("ThisIsALongString".to_string()),
        );

        let result = process(table_name, span, &db, Some(columns));

        assert!(result.is_err());
    }
}
