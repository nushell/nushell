use crate::dataframe::values::{Column, NuDataFrame, NuExpression, NuLazyFrame};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct LazyFillNull;

impl Command for LazyFillNull {
    fn name(&self) -> &str {
        "fill-null"
    }

    fn usage(&self) -> &str {
        "Replaces NULL values with the given expression"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "fill",
                SyntaxShape::Any,
                "Expression to use to fill the null values",
            )
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Fills the null values by 0",
            example: "[1 2 2 3 3] | into df | shift 2 | fill-null 0",
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "0".to_string(),
                    vec![
                        Value::test_int(0),
                        Value::test_int(0),
                        Value::test_int(1),
                        Value::test_int(2),
                        Value::test_int(2),
                    ],
                )])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let fill: Value = call.req(engine_state, stack, 0)?;
        let value = input.into_value(call.head);

        if NuExpression::can_downcast(&value) {
            let expr = NuExpression::try_from_value(value)?;
            let fill = NuExpression::try_from_value(fill)?.into_polars();
            let expr: NuExpression = expr.into_polars().fill_null(fill).into();

            Ok(PipelineData::Value(
                NuExpression::into_value(expr, call.head),
                None,
            ))
        } else {
            let lazy = NuLazyFrame::try_from_value(value)?;
            let expr = NuExpression::try_from_value(fill)?.into_polars();
            let lazy = NuLazyFrame::new(lazy.from_eager, lazy.into_polars().fill_null(expr));

            Ok(PipelineData::Value(lazy.into_value(call.head)?, None))
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::super::series::Shift;
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(LazyFillNull {}), Box::new(Shift {})])
    }
}
