use crate::database::values::dsl::ExprDb;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
};

#[derive(Clone)]
pub struct FieldExpr;

impl Command for FieldExpr {
    fn name(&self) -> &str {
        "field"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("name", SyntaxShape::String, "column name")
            .input_type(Type::Any)
            .output_type(Type::Custom("db-expression".into()))
            .category(Category::Custom("db-expression".into()))
    }

    fn usage(&self) -> &str {
        "Creates column expression for database"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "column", "expression"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates a named field expression",
            example: "field name_1 | into nu",
            result: Some(Value::Record {
                cols: vec!["value".into(), "quoted_style".into()],
                vals: vec![
                    Value::String {
                        val: "name_1".into(),
                        span: Span::test_data(),
                    },
                    Value::String {
                        val: "None".into(),
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            }),
        }]
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::database::test_database::test_database;

    #[test]
    fn test_examples() {
        test_database(vec![Box::new(FieldExpr {})])
    }
}
