use crate::dataframe::values::NuExpression;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use polars::prelude::col;

#[derive(Clone)]
pub struct ExprCol;

impl Command for ExprCol {
    fn name(&self) -> &str {
        "dfr col"
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
            .category(Category::Custom("expressions".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates a named column expression and converts it to a nu object",
            example: "dfr col col_a | dfr to-nu",
            result: Some(Value::Record {
                cols: vec!["expr".into(), "value".into()],
                vals: vec![
                    Value::String {
                        val: "column".into(),
                        span: Span::test_data(),
                    },
                    Value::String {
                        val: "col_a".into(),
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
    use super::super::super::super::test_dataframe::test_dataframe;
    use super::super::super::ExprToNu;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(ExprCol {}), Box::new(ExprToNu {})])
    }
}
