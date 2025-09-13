use crate::database::{MEMORY_DB, SQLiteDatabase};
use nu_engine::command_prelude::*;
use nu_protocol::Signals;

#[derive(Clone)]
pub struct StorDelete;

impl Command for StorDelete {
    fn name(&self) -> &str {
        "stor delete"
    }

    fn signature(&self) -> Signature {
        Signature::build("stor delete")
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .required_named(
                "table-name",
                SyntaxShape::String,
                "name of the table you want to delete or delete from",
                Some('t'),
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
        "Delete a table or specified rows in the in-memory sqlite database."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["sqlite", "remove", "table", "saving", "drop"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Delete a table from the in-memory sqlite database",
                example: "stor delete --table-name nudb",
                result: None,
            },
            Example {
                description: "Delete some rows from the in-memory sqlite database with a where clause",
                example: "stor delete --table-name nudb --where-clause \"int1 == 5\"",
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
        // For dropping/deleting an entire table
        let table_name_opt: Option<String> = call.get_flag(engine_state, stack, "table-name")?;

        // For deleting rows from a table
        let where_clause_opt: Option<String> =
            call.get_flag(engine_state, stack, "where-clause")?;

        if table_name_opt.is_none() && where_clause_opt.is_none() {
            return Err(ShellError::MissingParameter {
                param_name: "requires at least one of table-name or where-clause".into(),
                span,
            });
        }

        if table_name_opt.is_none() && where_clause_opt.is_some() {
            return Err(ShellError::MissingParameter {
                param_name: "using the where-clause requires the use of a table-name".into(),
                span,
            });
        }

        // Open the in-mem database
        let db = Box::new(SQLiteDatabase::new(
            std::path::Path::new(MEMORY_DB),
            Signals::empty(),
        ));

        if let Some(new_table_name) = table_name_opt
            && let Ok(conn) = db.open_connection()
        {
            let sql_stmt = match where_clause_opt {
                None => {
                    // We're deleting an entire table
                    format!("DROP TABLE {new_table_name}")
                }
                Some(where_clause) => {
                    // We're just deleting some rows
                    let mut delete_stmt = format!("DELETE FROM {new_table_name} ");

                    // Yup, this is a bit janky, but I'm not sure a better way to do this without having
                    // --and and --or flags as well as supporting ==, !=, <>, is null, is not null, etc.
                    // and other sql syntax. So, for now, just type a sql where clause as a string.
                    delete_stmt.push_str(&format!("WHERE {where_clause}"));
                    delete_stmt
                }
            };

            // dbg!(&sql_stmt);
            conn.execute(&sql_stmt, [])
                .map_err(|err| ShellError::GenericError {
                    error: "Failed to delete using the SQLite connection in memory from delete.rs."
                        .into(),
                    msg: err.to_string(),
                    span: Some(Span::test_data()),
                    help: None,
                    inner: vec![],
                })?;
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

        test_examples(StorDelete {})
    }
}
