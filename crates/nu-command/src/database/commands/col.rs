use crate::database::values::dsl::ExprDb;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct ColExpr;

impl Command for ColExpr {
    fn name(&self) -> &str {
        "db col"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("name", SyntaxShape::String, "column name")
            .category(Category::Custom("database".into()))
    }

    fn usage(&self) -> &str {
        "Creates column expression for database"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates a named column expression",
            example: "db col name_1",
            result: None,
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "column", "expression"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let value: Value = call.req(engine_state, stack, 0)?;
        let expression = ExprDb::try_from_value(&value)?;

        Ok(expression.into_value(call.head).into_pipeline_data())
    }
}
