use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature,
};

use super::super::SQLiteDatabase;

#[derive(Clone)]
pub struct DescribeDb;

impl Command for DescribeDb {
    fn name(&self) -> &str {
        "db describe"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Custom("database".into()))
    }

    fn usage(&self) -> &str {
        "Describes connection and query of the DB object"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Describe SQLite database constructed query",
            example: "db open foo.db | db select table_1 | db describe",
            result: None,
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "SQLite"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let db = SQLiteDatabase::try_from_pipeline(input, call.head)?;
        Ok(db.describe(call.head).into_pipeline_data())
    }
}
