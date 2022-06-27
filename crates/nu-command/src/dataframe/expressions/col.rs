use crate::dataframe::values::NuExpression;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use polars::prelude::col;

#[derive(Clone)]
pub struct ExprCol;

impl Command for ExprCol {
    fn name(&self) -> &str {
        "col"
    }

    fn usage(&self) -> &str {
        "Creates a named column expression"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "column name",
                SyntaxShape::String,
                "Name of column to be used",
            )
            .input_type(Type::Any)
            .output_type(Type::Custom("expression".into()))
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates a named column expression and converts it to a nu object",
            example: "col a | into nu",
            result: Some(Value::Record {
                cols: vec!["expr".into(), "value".into()],
                vals: vec![
                    Value::String {
                        val: "column".into(),
                        span: Span::test_data(),
                    },
                    Value::String {
                        val: "a".into(),
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
        let name: String = call.req(engine_state, stack, 0)?;
        let expr: NuExpression = col(name.as_str()).into();

        Ok(PipelineData::Value(expr.into_value(call.head), None))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;
    use crate::dataframe::expressions::as_nu::ExprAsNu;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(ExprCol {}), Box::new(ExprAsNu {})])
    }
}
