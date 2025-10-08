use crate::{
    PolarsPlugin,
    values::{
        CustomValueSupport, NuExpression, PolarsPluginObject, PolarsPluginType, cant_convert_err,
    },
};

use super::super::super::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Value,
};
use polars::prelude::{IntoSeries, StringNameSpaceImpl};

#[derive(Clone)]
pub struct ToUpperCase;

impl PluginCommand for ToUpperCase {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars uppercase"
    }

    fn description(&self) -> &str {
        "Uppercase the strings in the column."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["capitalize, caps, capital"]
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (
                    PolarsPluginType::NuDataFrame.into(),
                    PolarsPluginType::NuDataFrame.into(),
                ),
                (
                    PolarsPluginType::NuLazyFrame.into(),
                    PolarsPluginType::NuLazyFrame.into(),
                ),
                (
                    PolarsPluginType::NuExpression.into(),
                    PolarsPluginType::NuExpression.into(),
                ),
            ])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Modifies strings in a column to uppercase",
                example: "[[a]; [Abc]] | polars into-df | polars select (polars col a | polars uppercase) | polars collect",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "a".to_string(),
                            vec![Value::test_string("ABC")],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Modifies strings to uppercase",
                example: "[Abc aBc abC] | polars into-df | polars uppercase",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "0".to_string(),
                            vec![
                                Value::test_string("ABC"),
                                Value::test_string("ABC"),
                                Value::test_string("ABC"),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let value = input.into_value(call.head)?;
        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuDataFrame(df) => command_df(plugin, engine, call, df),
            PolarsPluginObject::NuLazyFrame(lazy) => {
                command_df(plugin, engine, call, lazy.collect(call.head)?)
            }
            PolarsPluginObject::NuExpression(expr) => command_expr(plugin, engine, call, expr),
            _ => Err(cant_convert_err(
                &value,
                &[
                    PolarsPluginType::NuDataFrame,
                    PolarsPluginType::NuLazyFrame,
                    PolarsPluginType::NuExpression,
                ],
            )),
        }
        .map_err(LabeledError::from)
        .map(|pd| pd.set_metadata(metadata))
    }
}

fn command_expr(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    expr: NuExpression,
) -> Result<PipelineData, ShellError> {
    let res: NuExpression = expr.into_polars().str().to_uppercase().into();
    res.to_pipeline_data(plugin, engine, call.head)
}

fn command_df(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let series = df.as_series(call.head)?;

    let casted = series.str().map_err(|e| ShellError::GenericError {
        error: "Error casting to string".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: Some("The str-slice command can only be used with string columns".into()),
        inner: vec![],
    })?;

    let mut res = casted.to_uppercase();
    res.rename(series.name().clone());

    let df = NuDataFrame::try_from_series_vec(vec![res.into_series()], call.head)?;
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ToUpperCase)
    }
}
