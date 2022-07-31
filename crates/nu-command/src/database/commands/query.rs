use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Spanned, SyntaxShape,
    Type,
};

use super::super::SQLiteDatabase;

#[derive(Clone)]
pub struct QueryDb;

impl Command for QueryDb {
    fn name(&self) -> &str {
        "query db"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "SQL",
                SyntaxShape::String,
                "SQL to execute against the database",
            )
            .input_type(Type::Any)
            .output_type(Type::Any)
            .category(Category::Custom("database".into()))
    }

    fn usage(&self) -> &str {
        "Query a database using SQL."
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Execute SQL against a SQLite database",
            example: r#"open foo.db | query db "SELECT * FROM Bar""#,
            result: None,
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "SQLite"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let sql: Spanned<String> = call.req(engine_state, stack, 0)?;

        let db = SQLiteDatabase::try_from_pipeline(input, call.head)?;
        db.query(&sql, call.head)
            .map(IntoPipelineData::into_pipeline_data)
    }
}
