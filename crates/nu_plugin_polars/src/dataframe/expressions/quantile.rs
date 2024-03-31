use crate::{
    dataframe::values::{Column, NuDataFrame, NuExpression},
    values::{to_pipeline_data, CustomValueSupport},
    PolarsPlugin,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, Signature, Span, SyntaxShape, Type, Value,
};
use polars::prelude::{lit, QuantileInterpolOptions};

#[derive(Clone)]
pub struct ExprQuantile;

impl PluginCommand for ExprQuantile {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars expr-quantile"
    }

    fn usage(&self) -> &str {
        "Aggregates the columns to the selected quantile."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "quantile",
                SyntaxShape::Number,
                "quantile value for quantile operation",
            )
            .input_output_type(
                Type::Custom("expression".into()),
                Type::Custom("expression".into()),
            )
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Quantile aggregation for a group-by",
            example: r#"[[a b]; [one 2] [one 4] [two 1]]
    | polars into-df
    | polars group-by a
    | polars agg (polars col b | polars expr-quantile 0.5)"#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "a".to_string(),
                            vec![Value::test_string("one"), Value::test_string("two")],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::test_float(4.0), Value::test_float(1.0)],
                        ),
                    ],
                    None,
                )
                .expect("simple df for test should not fail")
                .base_value(Span::test_data())
                .expect("rendering base value should not fail"),
            ),
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["statistics", "percentile", "distribution"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let value = input.into_value(call.head);
        let quantile: f64 = call.req(0)?;

        let expr = NuExpression::try_from_value(plugin, &value)?;
        let expr: NuExpression = expr
            .to_polars()
            .quantile(lit(quantile), QuantileInterpolOptions::default())
            .into();
        to_pipeline_data(plugin, engine, call.head, expr).map_err(LabeledError::from)
    }
}

// todo: fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//     use crate::dataframe::lazy::aggregate::LazyAggregate;
//     use crate::dataframe::lazy::groupby::ToLazyGroupBy;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![
//             Box::new(ExprQuantile {}),
//             Box::new(LazyAggregate {}),
//             Box::new(ToLazyGroupBy {}),
//         ])
//     }
// }
