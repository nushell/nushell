use crate::database::{SQLiteDatabase, MEMORY_DB};
use nu_engine::command_prelude::*;

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

                    create_stmt.push_str(") VALUES ( ");
                    let vals = record.values();
                    vals.for_each(|val| match val {
                        Value::Int { val, .. } => {
                            create_stmt.push_str(&format!("{}, ", val));
                        }
                        Value::Float { val, .. } => {
                            create_stmt.push_str(&format!("{}, ", val));
                        }
                        Value::String { val, .. } => {
                            create_stmt.push_str(&format!("'{}', ", val));
                        }
                        Value::Date { val, .. } => {
                            create_stmt.push_str(&format!("'{}', ", val));
                        }
                        Value::Bool { val, .. } => {
                            create_stmt.push_str(&format!("{}, ", val));
                        }
                        _ => {
                            // return Err(ShellError::UnsupportedInput {
                            //     msg: format!("{} is not a valid datepart, expected one of year, month, day, hour, minute, second, millisecond, microsecond, nanosecond", part.item),
                            //     input: "value originates from here".to_string(),
                            //     msg_span: span,
                            //     input_span: val.span(),
                            // });
                        }
                    });
                    if create_stmt.ends_with(", ") {
                        create_stmt.pop();
                        create_stmt.pop();
                    }

                    create_stmt.push(')');

                    // dbg!(&create_stmt);

                    conn.execute(&create_stmt, [])
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
                        span: call.head,
                    });
                }
            };
        }
        // dbg!(db.clone());
        Ok(Value::custom(db, span).into_pipeline_data())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(StorInsert {})
    }
}
