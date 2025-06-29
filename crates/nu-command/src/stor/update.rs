use crate::database::{MEMORY_DB, SQLiteDatabase, values_to_sql};
use nu_engine::command_prelude::*;
use nu_protocol::Signals;
use rusqlite::params_from_iter;

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
                // FIXME Type::Any input added to disable pipeline input type checking, as run-time checks can raise undesirable type errors
                // which aren't caught by the parser. see https://github.com/nushell/nushell/pull/14922 for more details
                (Type::Any, Type::table()),
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

    fn description(&self) -> &str {
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
        let mut update_stmt = format!("UPDATE {new_table_name} ");

        update_stmt.push_str("SET ");
        let mut placeholders: Vec<String> = Vec::new();

        for (index, (key, _)) in record.iter().enumerate() {
            placeholders.push(format!("{} = ?{}", key, index + 1));
        }
        update_stmt.push_str(&placeholders.join(", "));

        // Yup, this is a bit janky, but I'm not sure a better way to do this without having
        // --and and --or flags as well as supporting ==, !=, <>, is null, is not null, etc.
        // and other sql syntax. So, for now, just type a sql where clause as a string.
        if let Some(where_clause) = where_clause_opt {
            update_stmt.push_str(&format!(" WHERE {}", where_clause.item));
        }
        // dbg!(&update_stmt);

        // Get the params from the passed values
        let params = values_to_sql(record.values().cloned())?;

        conn.execute(&update_stmt, params_from_iter(params))
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
