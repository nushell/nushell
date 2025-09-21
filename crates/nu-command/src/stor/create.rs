use crate::database::{MEMORY_DB, SQLiteDatabase};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct StorCreate;

impl Command for StorCreate {
    fn name(&self) -> &str {
        "stor create"
    }

    fn signature(&self) -> Signature {
        Signature::build("stor create")
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .required_named(
                "table-name",
                SyntaxShape::String,
                "name of the table you want to create",
                Some('t'),
            )
            .required_named(
                "columns",
                SyntaxShape::Record(vec![]),
                "a record of column names and datatypes",
                Some('c'),
            )
            .allow_variants_without_examples(true)
            .category(Category::Database)
    }

    fn description(&self) -> &str {
        "Create a table in the in-memory sqlite database."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["sqlite", "storing", "table"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Create an in-memory sqlite database with specified table name, column names, and column data types",
                example: "stor create --table-name nudb --columns {bool1: bool, int1: int, float1: float, str1: str, datetime1: datetime}",
                result: None,
            },
            Example {
                description: "Create an in-memory sqlite database with a json column",
                example: "stor create --table-name files_with_md --columns {file: str, metadata: jsonb}",
                result: None,
            },
        ]
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
        let columns: Option<Record> = call.get_flag(engine_state, stack, "columns")?;
        let db = Box::new(SQLiteDatabase::new(
            std::path::Path::new(MEMORY_DB),
            engine_state.signals().clone(),
        ));

        process(table_name, span, &db, columns)?;
        // dbg!(db.clone());
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
                let mut create_stmt = format!("CREATE TABLE {new_table_name} ( ");
                for (column_name, column_datatype) in record {
                    match column_datatype.coerce_str()?.to_lowercase().as_ref() {
                        "int" => {
                            create_stmt.push_str(&format!("{column_name} INTEGER, "));
                        }
                        "float" => {
                            create_stmt.push_str(&format!("{column_name} REAL, "));
                        }
                        "str" => {
                            create_stmt.push_str(&format!("{column_name} VARCHAR(255), "));
                        }

                        "bool" => {
                            create_stmt.push_str(&format!("{column_name} BOOLEAN, "));
                        }
                        "datetime" => {
                            create_stmt.push_str(&format!(
                                "{column_name} DATETIME DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')), "
                            ));
                        }
                        "json" => {
                            create_stmt.push_str(&format!("{column_name} JSON, "));
                        }
                        "jsonb" => {
                            create_stmt.push_str(&format!("{column_name} JSONB, "));
                        }

                        _ => {
                            return Err(ShellError::UnsupportedInput {
                                msg: "Unsupported column data type. Please use: int, float, str, bool, datetime, json, jsonb".into(),
                                input: format!("{column_datatype:?}"),
                                msg_span: column_datatype.span(),
                                input_span: column_datatype.span(),
                            });
                        }
                    }
                }
                if create_stmt.ends_with(", ") {
                    create_stmt.pop();
                    create_stmt.pop();
                }
                create_stmt.push_str(" )");

                // dbg!(&create_stmt);

                conn.execute(&create_stmt, [])
                    .map_err(|err| ShellError::GenericError {
                        error: "Failed to open SQLite connection in memory from create".into(),
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
    Ok(())
}

#[cfg(test)]
mod test {
    use nu_protocol::Signals;

    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(StorCreate {})
    }

    #[test]
    fn test_process_with_valid_parameters() {
        let table_name = Some("test_table".to_string());
        let span = Span::unknown();
        let db = Box::new(SQLiteDatabase::new(
            std::path::Path::new(MEMORY_DB),
            Signals::empty(),
        ));
        let mut columns = Record::new();
        columns.insert(
            "int_column".to_string(),
            Value::test_string("int".to_string()),
        );

        let result = process(table_name, span, &db, Some(columns));

        assert!(result.is_ok());
    }

    #[test]
    fn test_process_with_missing_table_name() {
        let table_name = None;
        let span = Span::unknown();
        let db = Box::new(SQLiteDatabase::new(
            std::path::Path::new(MEMORY_DB),
            Signals::empty(),
        ));
        let mut columns = Record::new();
        columns.insert(
            "int_column".to_string(),
            Value::test_string("int".to_string()),
        );

        let result = process(table_name, span, &db, Some(columns));

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("requires at table name")
        );
    }

    #[test]
    fn test_process_with_missing_columns() {
        let table_name = Some("test_table".to_string());
        let span = Span::unknown();
        let db = Box::new(SQLiteDatabase::new(
            std::path::Path::new(MEMORY_DB),
            Signals::empty(),
        ));

        let result = process(table_name, span, &db, None);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("requires at least one column")
        );
    }

    #[test]
    fn test_process_with_unsupported_column_data_type() {
        let table_name = Some("test_table".to_string());
        let span = Span::unknown();
        let db = Box::new(SQLiteDatabase::new(
            std::path::Path::new(MEMORY_DB),
            Signals::empty(),
        ));
        let mut columns = Record::new();
        let column_datatype = "bogus_data_type".to_string();
        columns.insert(
            "column0".to_string(),
            Value::test_string(column_datatype.clone()),
        );

        let result = process(table_name, span, &db, Some(columns));

        assert!(result.is_err());

        let expected_err = ShellError::UnsupportedInput {
            msg: "unsupported column data type".into(),
            input: format!("{:?}", column_datatype.clone()),
            msg_span: Span::test_data(),
            input_span: Span::test_data(),
        };
        assert_eq!(result.unwrap_err().to_string(), expected_err.to_string());
    }
}
