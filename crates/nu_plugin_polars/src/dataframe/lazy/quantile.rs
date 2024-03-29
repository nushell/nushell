use crate::{
    dataframe::values::{Column, NuDataFrame, NuLazyFrame},
    values::{
        cant_convert_err, to_pipeline_data, CustomValueSupport, PolarsPluginObject,
        PolarsPluginType,
    },
    Cacheable, PolarsPlugin,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::prelude::{lit, QuantileInterpolOptions};

#[derive(Clone)]
pub struct LazyQuantile;

impl PluginCommand for LazyQuantile {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars quantile"
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
            example: "[[a b]; [6 2] [1 4] [4 1]] | polars into-df | polars quantile 0.5",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new("a".to_string(), vec![Value::test_float(4.0)]),
                        Column::new("b".to_string(), vec![Value::test_float(2.0)]),
                    ],
                    None,
                )
                .expect("simple df for test should not fail")
                .base_value(Span::test_data())
                .expect("rendering base value should not fail"),
            ),
        }]
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

        let lazy = match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuDataFrame(df) => df.lazy(),
            PolarsPluginObject::NuLazyFrame(lazy) => lazy,
            _ => Err(cant_convert_err(
                &value,
                &[PolarsPluginType::NuLazyFrame, PolarsPluginType::NuDataFrame],
            ))?,
        };
        let lazy = NuLazyFrame::new(
            lazy.from_eager,
            lazy.to_polars()
                .quantile(lit(quantile), QuantileInterpolOptions::default())
                .map_err(|e| ShellError::GenericError {
                    error: "Dataframe Error".into(),
                    msg: e.to_string(),
                    help: None,
                    span: None,
                    inner: vec![],
                })?,
        );

        to_pipeline_data(plugin, engine, call.head, lazy).map_err(LabeledError::from)
    }
}

// todo: fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![Box::new(LazyQuantile {})])
//     }
// }
