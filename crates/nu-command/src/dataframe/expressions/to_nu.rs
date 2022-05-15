use super::super::values::NuExpression;

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Value,
};

#[derive(Clone)]
pub struct ExprToNu;

impl Command for ExprToNu {
    fn name(&self) -> &str {
        "dfr to-nu"
    }

    fn usage(&self) -> &str {
        "Convert expression to a nu value for access and exploration"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Custom("expressions".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Convert a col expression into a nushell value",
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
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let expr = NuExpression::try_from_pipeline(input, call.head)?;
        let value = expr.to_value(call.head);

        Ok(PipelineData::Value(value, None))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::super::ExprCol;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(ExprToNu {}), Box::new(ExprCol {})])
    }
}
