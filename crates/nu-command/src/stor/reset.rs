use crate::database::{MEMORY_DB, SQLiteDatabase};
use nu_engine::command_prelude::*;
use nu_protocol::Signals;

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

    fn description(&self) -> &str {
        "Reset the in-memory database by dropping all tables."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["sqlite", "remove", "table", "saving", "drop"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
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
        let db = Box::new(SQLiteDatabase::new(
            std::path::Path::new(MEMORY_DB),
            Signals::empty(),
        ));

        if let Ok(conn) = db.open_connection() {
            conn.execute("PRAGMA foreign_keys = OFF", [])
                .map_err(|err| ShellError::GenericError {
                    error: "Failed to turn off foreign_key protections for reset".into(),
                    msg: err.to_string(),
                    span: Some(Span::test_data()),
                    help: None,
                    inner: vec![],
                })?;
            db.drop_all_tables(&conn)
                .map_err(|err| ShellError::GenericError {
                    error: "Failed to drop all tables in memory from reset".into(),
                    msg: err.to_string(),
                    span: Some(Span::test_data()),
                    help: None,
                    inner: vec![],
                })?;
            conn.execute("PRAGMA foreign_keys = ON", [])
                .map_err(|err| ShellError::GenericError {
                    error: "Failed to turn on foreign_key protections for reset".into(),
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
