use super::super::values::NuExpression;

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct ExprAlias;

impl Command for ExprAlias {
    fn name(&self) -> &str {
        "as"
    }

    fn usage(&self) -> &str {
        "Creates an alias expression"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "Alias name",
                SyntaxShape::String,
                "Alias name for the expression",
            )
            .input_type(Type::Custom("expression".into()))
            .output_type(Type::Custom("expression".into()))
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates and alias expression",
            example: "col a | as new_a | into nu",
            result: {
                let cols = vec!["expr".into(), "value".into()];
                let expr = Value::test_string("column");
                let value = Value::test_string("a");
                let expr = Value::Record {
                    cols,
                    vals: vec![expr, value],
                    span: Span::test_data(),
                };

                let cols = vec!["expr".into(), "alias".into()];
                let value = Value::test_string("new_a");

                let record = Value::Record {
                    cols,
                    vals: vec![expr, value],
                    span: Span::test_data(),
                };

                Some(record)
            },
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let alias: String = call.req(engine_state, stack, 0)?;

        let expr = NuExpression::try_from_pipeline(input, call.head)?;
        let expr: NuExpression = expr.into_polars().alias(alias.as_str()).into();

        Ok(PipelineData::Value(
            NuExpression::into_value(expr, call.head),
            None,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;
    use crate::dataframe::expressions::ExprAsNu;
    use crate::dataframe::expressions::ExprCol;

    #[test]
    fn test_examples() {
        test_dataframe(vec![
            Box::new(ExprAlias {}),
            Box::new(ExprCol {}),
            Box::new(ExprAsNu {}),
        ])
    }
}
