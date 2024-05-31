use crate::database::{SQLiteDatabase, MEMORY_DB};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct StorReset;

impl Command for StorReset {
    fn name(&self) -> &str {
        "stor reset"
    }

    fn signature(&self) -> Signature {
        Signature::build("stor reset")
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .allow_variants_without_examples(true)
            .category(Category::Database)
    }

    fn usage(&self) -> &str {
        "Reset the in-memory database by dropping all tables."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["sqlite", "remove", "table", "saving", "drop"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Reset the in-memory sqlite database",
            example: "stor reset",
            result: None,
        }]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;

        // Open the in-mem database
        let db = Box::new(SQLiteDatabase::new(std::path::Path::new(MEMORY_DB), None));

        if let Ok(conn) = db.open_connection() {
            db.drop_all_tables(&conn)
                .map_err(|err| ShellError::GenericError {
                    error: "Failed to open SQLite connection in memory from reset".into(),
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

        test_examples(StorReset {})
    }
}
