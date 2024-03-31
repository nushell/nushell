use crate::dataframe::values::{Column, NuDataFrame, NuLazyFrame};
use nu_engine::command_prelude::*;

use polars::prelude::{lit, QuantileInterpolOptions};

#[derive(Clone)]
pub struct LazyQuantile;

impl Command for LazyQuantile {
    fn name(&self) -> &str {
        "dfr quantile"
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
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "quantile value from columns in a dataframe",
            example: "[[a b]; [6 2] [1 4] [4 1]] | dfr into-df | dfr quantile 0.5",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new("a".to_string(), vec![Value::test_float(4.0)]),
                        Column::new("b".to_string(), vec![Value::test_float(2.0)]),
                    ],
                    None,
                )
                .expect("simple df for test should not fail")
                .into_value(FutureSpanId::test_data()),
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

        let lazy = NuLazyFrame::try_from_value(value)?;
        let lazy = NuLazyFrame::new(
            lazy.from_eager,
            lazy.into_polars()
                .quantile(lit(quantile), QuantileInterpolOptions::default())
                .map_err(|e| ShellError::GenericError {
                    error: "Dataframe Error".into(),
                    msg: e.to_string(),
                    help: None,
                    span: None,
                    inner: vec![],
                })?,
        );

        Ok(PipelineData::Value(lazy.into_value(call.head)?, None))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(LazyQuantile {})])
    }
}
