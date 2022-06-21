use super::super::SQLiteDatabase;

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Type,
};

#[derive(Clone)]
pub struct ToDataBase;

impl Command for ToDataBase {
    fn name(&self) -> &str {
        "into db"
    }

    fn usage(&self) -> &str {
        "Converts into an open db connection"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Custom("database".into()))
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "into", "db"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Converts an open file into a db object",
            example: "open db.mysql | into db",
            result: None,
        }]
    }

    fn input_type(&self) -> Type {
        Type::Any
    }

    fn output_type(&self) -> Type {
        Type::Custom("database".into())
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let db = SQLiteDatabase::try_from_pipeline(input, call.head)?;
        Ok(db.into_value(call.head).into_pipeline_data())
    }
}
