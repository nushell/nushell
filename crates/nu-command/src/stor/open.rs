use crate::database::{MEMORY_DB, SQLiteDatabase};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct StorOpen;

impl Command for StorOpen {
    fn name(&self) -> &str {
        "stor open"
    }

    fn signature(&self) -> Signature {
        Signature::build("stor open")
            .input_output_types(vec![(Type::Nothing, Type::Custom("SQLiteDatabase".into()))])
            .allow_variants_without_examples(true)
            .category(Category::Database)
    }

    fn description(&self) -> &str {
        "Opens the in-memory sqlite database."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["sqlite", "storing", "access"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Open the in-memory sqlite database",
            example: "stor open",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // TODO: Think about adding the following functionality
        // * stor open --table-name my_table_name
        //   It returns the output of `select * from my_table_name`

        // Just create an empty database with MEMORY_DB and nothing else
        let db = Box::new(SQLiteDatabase::new(
            std::path::Path::new(MEMORY_DB),
            engine_state.signals().clone(),
        ));

        Ok(db.into_value(call.head).into_pipeline_data())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use nu_test_support::Result;
    use nu_test_support::prelude::*;

    #[test]
    fn test_examples() -> Result {
        test().examples(StorOpen)
    }

    #[test]
    #[exp(nu_experimental::ENFORCE_RUNTIME_ANNOTATIONS)]
    fn correct_return_ty() -> Result {
        let () = test().run("let db = stor open")?;
        Ok(())
    }
}
