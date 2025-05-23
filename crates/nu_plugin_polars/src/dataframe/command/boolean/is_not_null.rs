use crate::{
    PolarsPlugin,
    values::{CustomValueSupport, PolarsPluginObject, PolarsPluginType, cant_convert_err},
};

use super::super::super::values::{Column, NuDataFrame, NuExpression};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Type, Value,
};
use polars::prelude::IntoSeries;

#[derive(Clone)]
pub struct IsNotNull;

impl PluginCommand for IsNotNull {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars is-not-null"
    }

    fn description(&self) -> &str {
        "Creates mask where value is not null."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (
                    Type::Custom("expression".into()),
                    Type::Custom("expression".into()),
                ),
                (
                    Type::Custom("dataframe".into()),
                    Type::Custom("dataframe".into()),
                ),
            ])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create mask where values are not null",
                example: r#"let s = ([5 6 0 8] | polars into-df);
    let res = ($s / $s);
    $res | polars is-not-null"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "is_not_null".to_string(),
                            vec![
                                Value::test_bool(true),
                                Value::test_bool(true),
                                Value::test_bool(false),
                                Value::test_bool(true),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Creates a is not null expression from a column",
                example: "polars col a | polars is-not-null",
                result: None,
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
            PolarsPluginObject::NuDataFrame(df) => command(plugin, engine, call, df),
            PolarsPluginObject::NuLazyFrame(lazy) => {
                command(plugin, engine, call, lazy.collect(call.head)?)
            }
            PolarsPluginObject::NuExpression(expr) => {
                let expr: NuExpression = expr.into_polars().is_not_null().into();
                expr.to_pipeline_data(plugin, engine, call.head)
            }
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

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let mut res = df.as_series(call.head)?.is_not_null();
    res.rename("is_not_null".into());

    let df = NuDataFrame::try_from_series_vec(vec![res.into_series()], call.head)?;
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&IsNotNull)
    }
}
