use crate::database::{SQLiteDatabase, MEMORY_DB};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, Record, ShellError, Signature, Span,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct StorCreate;

impl Command for StorCreate {
    fn name(&self) -> &str {
        "stor create"
    }

    fn signature(&self) -> Signature {
        Signature::build("stor create")
            .input_output_types(vec![(Type::Nothing, Type::Table(vec![]))])
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
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Create a table in the in-memory sqlite database"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["sqlite", "storing", "table"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Create an in-memory sqlite database with specified table name, column names, and column data types",
            example: "stor create --table-name nudb --columns {bool1: bool, int1: int, float1: float, str1: str, datetime1: datetime}",
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
        let columns: Option<Record> = call.get_flag(engine_state, stack, "columns")?;
        let db = Box::new(SQLiteDatabase::new(std::path::Path::new(MEMORY_DB), None));

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
                    let mut create_stmt = format!(
                        "CREATE TABLE {} ( id INTEGER NOT NULL PRIMARY KEY, ",
                        new_table_name
                    );
                    for (column_name, column_datatype) in record {
                        match column_datatype.as_string()?.as_str() {
                            "int" => {
                                create_stmt.push_str(&format!("{} INTEGER, ", column_name));
                            }
                            "float" => {
                                create_stmt.push_str(&format!("{} REAL, ", column_name));
                            }
                            "str" => {
                                create_stmt.push_str(&format!("{} VARCHAR(255), ", column_name));
                            }

                            "bool" => {
                                create_stmt.push_str(&format!("{} BOOLEAN, ", column_name));
                            }
                            "datetime" => {
                                create_stmt.push_str(&format!(
                                    "{} DATETIME DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')), ",
                                    column_name
                                ));
                            }

                            _ => {
                                return Err(ShellError::UnsupportedInput {
                                    msg: "unsupported column data type".into(),
                                    input: format!("{:?}", column_datatype),
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

                    conn.execute(&create_stmt, []).map_err(|err| {
                        ShellError::GenericError(
                            "Failed to open SQLite connection in memory from create".into(),
                            err.to_string(),
                            Some(Span::test_data()),
                            None,
                            Vec::new(),
                        )
                    })?;
                }
                None => {
                    return Err(ShellError::MissingParameter {
                        param_name: "requires at least one column".into(),
                        span: call.head,
                    });
                }
            };
        }
        // dbg!(db.clone());
        Ok(Value::custom_value(db, span).into_pipeline_data())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(StorCreate {})
    }
}
