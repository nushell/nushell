use super::super::values::NuExpression;

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct ExprAsNu;

impl Command for ExprAsNu {
    fn name(&self) -> &str {
        "into nu"
    }

    fn usage(&self) -> &str {
        "Convert expression into a nu value for access and exploration"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_type(Type::Custom("expression".into()))
            .output_type(Type::Any)
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Convert a col expression into a nushell value",
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
        test_dataframe(vec![Box::new(ExprAsNu {}), Box::new(ExprCol {})])
    }
}
