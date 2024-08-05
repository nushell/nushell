use crate::database::{SQLiteDatabase, MEMORY_DB};
use nu_engine::command_prelude::*;
use nu_protocol::Signals;

#[derive(Clone)]
pub struct StorUpdate;

impl Command for StorUpdate {
    fn name(&self) -> &str {
        "stor update"
    }

    fn signature(&self) -> Signature {
        Signature::build("stor update")
            .input_output_types(vec![
                (Type::Nothing, Type::table()),
                (Type::record(), Type::table()),
            ])
            .required_named(
                "table-name",
                SyntaxShape::String,
                "name of the table you want to insert into",
                Some('t'),
            )
            .named(
                "update-record",
                SyntaxShape::Record(vec![]),
                "a record of column names and column values to update in the specified table",
                Some('u'),
            )
            .named(
                "where-clause",
                SyntaxShape::String,
                "a sql string to use as a where clause without the WHERE keyword",
                Some('w'),
            )
            .allow_variants_without_examples(true)
            .category(Category::Database)
    }

    fn usage(&self) -> &str {
        "Update information in a specified table in the in-memory sqlite database."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["sqlite", "storing", "table", "saving", "changing"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
        Example {
            description: "Update the in-memory sqlite database",
            example: "stor update --table-name nudb --update-record {str1: nushell datetime1: 2020-04-17}",
            result: None,
        },
        Example {
            description: "Update the in-memory sqlite database with a where clause",
            example: "stor update --table-name nudb --update-record {str1: nushell datetime1: 2020-04-17} --where-clause \"bool1 = 1\"",
            result: None,
        },
        Example {
            description: "Update the in-memory sqlite database through pipeline input",
            example: "{str1: nushell datetime1: 2020-04-17} | stor update --table-name nudb",
            result: None,
        },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let table_name: Option<String> = call.get_flag(engine_state, stack, "table-name")?;
        let update_record: Option<Record> = call.get_flag(engine_state, stack, "update-record")?;
        let where_clause_opt: Option<Spanned<String>> =
            call.get_flag(engine_state, stack, "where-clause")?;

        // Open the in-mem database
        let db = Box::new(SQLiteDatabase::new(
            std::path::Path::new(MEMORY_DB),
            Signals::empty(),
        ));

        // Check if the record is being passed as input or using the update record parameter
        let columns = handle(span, update_record, input)?;

        process(table_name, span, &db, columns, where_clause_opt)?;

        Ok(Value::custom(db, span).into_pipeline_data())
    }
}

fn handle(
    span: Span,
    update_record: Option<Record>,
    input: PipelineData,
) -> Result<Record, ShellError> {
    match input {
        PipelineData::Empty => update_record.ok_or_else(|| ShellError::MissingParameter {
            param_name: "requires a record".into(),
            span,
        }),
        PipelineData::Value(value, ..) => {
            // Since input is being used, check if the data record parameter is used too
            if update_record.is_some() {
                return Err(ShellError::GenericError {
                    error: "Pipeline and Flag both being used".into(),
                    msg: "Use either pipeline input or '--update-record' parameter".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                });
            }
            match value {
                Value::Record { val, .. } => Ok(val.into_owned()),
                val => Err(ShellError::OnlySupportsThisInputType {
                    exp_input_type: "record".into(),
                    wrong_type: val.get_type().to_string(),
                    dst_span: Span::unknown(),
                    src_span: val.span(),
                }),
            }
        }
        _ => {
            if update_record.is_some() {
                return Err(ShellError::GenericError {
                    error: "Pipeline and Flag both being used".into(),
                    msg: "Use either pipeline input or '--update-record' parameter".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                });
            }
            Err(ShellError::OnlySupportsThisInputType {
                exp_input_type: "record".into(),
                wrong_type: "".into(),
                dst_span: span,
                src_span: span,
            })
        }
    }
}

fn process(
    table_name: Option<String>,
    span: Span,
    db: &SQLiteDatabase,
    record: Record,
    where_clause_opt: Option<Spanned<String>>,
) -> Result<(), ShellError> {
    if table_name.is_none() {
        return Err(ShellError::MissingParameter {
            param_name: "requires at table name".into(),
            span,
        });
    }
    let new_table_name = table_name.unwrap_or("table".into());
    if let Ok(conn) = db.open_connection() {
        let mut update_stmt = format!("UPDATE {} ", new_table_name);

        update_stmt.push_str("SET ");
        let vals = record.iter();
        vals.for_each(|(key, val)| match val {
            Value::Int { val, .. } => {
                update_stmt.push_str(&format!("{} = {}, ", key, val));
            }
            Value::Float { val, .. } => {
                update_stmt.push_str(&format!("{} = {}, ", key, val));
            }
            Value::String { val, .. } => {
                update_stmt.push_str(&format!("{} = '{}', ", key, val));
            }
            Value::Date { val, .. } => {
                update_stmt.push_str(&format!("{} = '{}', ", key, val));
            }
            Value::Bool { val, .. } => {
                update_stmt.push_str(&format!("{} = {}, ", key, val));
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
        if update_stmt.ends_with(", ") {
            update_stmt.pop();
            update_stmt.pop();
        }

        // Yup, this is a bit janky, but I'm not sure a better way to do this without having
        // --and and --or flags as well as supporting ==, !=, <>, is null, is not null, etc.
        // and other sql syntax. So, for now, just type a sql where clause as a string.
        if let Some(where_clause) = where_clause_opt {
            update_stmt.push_str(&format!(" WHERE {}", where_clause.item));
        }
        // dbg!(&update_stmt);

        conn.execute(&update_stmt, [])
            .map_err(|err| ShellError::GenericError {
                error: "Failed to open SQLite connection in memory from update".into(),
                msg: err.to_string(),
                span: Some(Span::test_data()),
                help: None,
                inner: vec![],
            })?;
    }
    // dbg!(db.clone());
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(StorUpdate {})
    }
}
