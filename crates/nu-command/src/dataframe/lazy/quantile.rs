use crate::dataframe::values::{Column, NuDataFrame, NuExpression, NuLazyFrame};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use polars::prelude::QuantileInterpolOptions;

#[derive(Clone)]
pub struct LazyQuantile;

impl Command for LazyQuantile {
    fn name(&self) -> &str {
        "dfr quantile"
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
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "quantile value from columns in a dataframe",
                example: "[[a b]; [6 2] [1 4] [4 1]] | dfr to-df | dfr quantile 0.5",
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new("a".to_string(), vec![Value::test_float(4.0)]),
                        Column::new("b".to_string(), vec![Value::test_float(2.0)]),
                    ])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Quantile aggregation for a group by",
                example: r#"[[a b]; [one 2] [one 4] [two 1]] 
    | dfr to-df 
    | dfr group-by a
    | dfr agg ("b" | dfr quantile 0.5)"#,
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
            },
        ]
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

        if NuExpression::can_downcast(&value) {
            let expr = NuExpression::try_from_value(value)?;
            let expr: NuExpression = expr
                .into_polars()
                .quantile(quantile, QuantileInterpolOptions::default())
                .into();

            Ok(PipelineData::Value(
                NuExpression::into_value(expr, call.head),
                None,
            ))
        } else {
            let lazy = NuLazyFrame::try_from_value(value)?;
            let lazy = NuLazyFrame::new(
                lazy.from_eager,
                lazy.into_polars()
                    .quantile(quantile, QuantileInterpolOptions::default()),
            );

            Ok(PipelineData::Value(lazy.into_value(call.head)?, None))
        }
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
            Box::new(LazyQuantile {}),
            Box::new(LazyAggregate {}),
            Box::new(ToLazyGroupBy {}),
        ])
    }
}
