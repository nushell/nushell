use crate::database::{MEMORY_DB, SQLiteDatabase};
use nu_engine::command_prelude::*;
use nu_protocol::Signals;

#[derive(Clone)]
pub struct StorExport;

impl Command for StorExport {
    fn name(&self) -> &str {
        "stor export"
    }

    fn signature(&self) -> Signature {
        Signature::build("stor export")
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .required_named(
                "file-name",
                SyntaxShape::String,
                "file name to export the sqlite in-memory database to",
                Some('f'),
            )
            .allow_variants_without_examples(true)
            .category(Category::Database)
    }

    fn description(&self) -> &str {
        "Export the in-memory sqlite database to a sqlite database file."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["sqlite", "save", "database", "saving", "file"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Export the in-memory sqlite database",
            example: "stor export --file-name nudb.sqlite",
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
        let file_name_opt: Option<String> = call.get_flag(engine_state, stack, "file-name")?;
        let file_name = match file_name_opt {
            Some(file_name) => file_name,
            None => {
                return Err(ShellError::MissingParameter {
                    param_name: "please supply a file name with the --file-name parameter".into(),
                    span,
                });
            }
        };

        // Open the in-mem database
        let db = Box::new(SQLiteDatabase::new(
            std::path::Path::new(MEMORY_DB),
            Signals::empty(),
        ));

        if let Ok(conn) = db.open_connection() {
            // This uses vacuum. I'm not really sure if this is the best way to do this.
            // I also added backup in the sqlitedatabase impl. If we have problems, we could switch to that.
            db.export_in_memory_database_to_file(&conn, file_name)
                .map_err(|err| ShellError::GenericError {
                    error: "Failed to open SQLite connection in memory from export".into(),
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

        test_examples(StorExport {})
    }
}
