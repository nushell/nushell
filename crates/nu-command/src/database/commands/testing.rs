use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Spanned, SyntaxShape,
    Value,
};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

#[derive(Clone)]
pub struct TestingDb;

impl Command for TestingDb {
    fn name(&self) -> &str {
        "testing-db"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "query",
                SyntaxShape::String,
                "SQL to execute to create the query object",
            )
            .category(Category::Custom("database".into()))
    }

    fn usage(&self) -> &str {
        "Temporal Command: Create query object"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "",
            example: "",
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
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let sql: Spanned<String> = call.req(engine_state, stack, 0)?;

        let dialect = GenericDialect {}; // or AnsiDialect, or your own dialect ...

        let ast = Parser::parse_sql(&dialect, sql.item.as_str()).map_err(|e| {
            ShellError::GenericError(
                "Error creating AST".into(),
                e.to_string(),
                Some(sql.span),
                None,
                Vec::new(),
            )
        })?;

        let value = match ast.get(0) {
            None => Value::nothing(call.head),
            Some(statement) => Value::String {
                val: format!("{:#?}", statement),
                span: call.head,
            },
        };

        Ok(value.into_pipeline_data())
    }
}
