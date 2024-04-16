use crate::{
    values::{cant_convert_err, CustomValueSupport, PolarsPluginObject, PolarsPluginType},
    PolarsPlugin,
};

use super::super::values::{Column, NuDataFrame, NuExpression};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct NUnique;

impl PluginCommand for NUnique {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars n-unique"
    }

    fn usage(&self) -> &str {
        "Counts unique values."
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
                description: "Counts unique values",
                example: "[1 1 2 2 3 3 4] | polars into-df | polars n-unique",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "count_unique".to_string(),
                            vec![Value::test_int(4)],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Creates a is n-unique expression from a column",
                example: "polars col a | polars n-unique",
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
        let value = input.into_value(call.head);

        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuDataFrame(df) => command(plugin, engine, call, df),
            PolarsPluginObject::NuLazyFrame(lazy) => {
                command(plugin, engine, call, lazy.collect(call.head)?)
            }
            PolarsPluginObject::NuExpression(expr) => {
                let expr: NuExpression = expr.to_polars().n_unique().into();
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
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let res = df
        .as_series(call.head)?
        .n_unique()
        .map_err(|e| ShellError::GenericError {
            error: "Error counting unique values".into(),
            msg: e.to_string(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?;

    let value = Value::int(res as i64, call.head);

    let df = NuDataFrame::try_from_columns(
        vec![Column::new("count_unique".to_string(), vec![value])],
        None,
    )?;
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&NUnique)
    }
}
