use crate::dataframe::values::{Column, NuDataFrame, NuExpression};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use polars::prelude::QuantileInterpolOptions;

#[derive(Clone)]
pub struct ExprQuantile;

impl Command for ExprQuantile {
    fn name(&self) -> &str {
        "quantile"
    }

    fn usage(&self) -> &str {
        "Aggregates the columns to the selected quantile"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "quantile",
                SyntaxShape::Number,
                "quantile value for quantile operation",
            )
            .input_type(Type::Custom("expression".into()))
            .output_type(Type::Custom("expression".into()))
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Quantile aggregation for a group-by",
            example: r#"[[a b]; [one 2] [one 4] [two 1]]
    | into df
    | group-by a
    | agg (col b | quantile 0.5)"#,
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "a".to_string(),
                        vec![Value::test_string("one"), Value::test_string("two")],
                    ),
                    Column::new(
                        "b".to_string(),
                        vec![Value::test_float(4.0), Value::test_float(1.0)],
                    ),
                ])
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
        let value = input.into_value(call.head);
        let quantile: f64 = call.req(engine_state, stack, 0)?;

        let expr = NuExpression::try_from_value(value)?;
        let expr: NuExpression = expr
            .into_polars()
            .quantile(quantile, QuantileInterpolOptions::default())
            .into();

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
    use crate::dataframe::lazy::aggregate::LazyAggregate;
    use crate::dataframe::lazy::groupby::ToLazyGroupBy;

    #[test]
    fn test_examples() {
        test_dataframe(vec![
            Box::new(ExprQuantile {}),
            Box::new(LazyAggregate {}),
            Box::new(ToLazyGroupBy {}),
        ])
    }
}
