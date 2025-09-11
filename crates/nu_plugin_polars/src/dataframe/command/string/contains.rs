use crate::{
    PolarsPlugin,
    values::{
        CustomValueSupport, NuExpression, PolarsPluginObject, PolarsPluginType, cant_convert_err,
    },
};

use super::super::super::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::prelude::{IntoSeries, StringNameSpaceImpl, lit};

#[derive(Clone)]
pub struct Contains;

impl PluginCommand for Contains {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars contains"
    }

    fn description(&self) -> &str {
        "Checks if a pattern is contained in a string."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "pattern",
                SyntaxShape::String,
                "Regex pattern to be searched",
            )
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

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Returns boolean indicating if pattern was found in a column",
                example: "let df = [[a]; [abc] [acb] [acb]] | polars into-df;
                let df2 = $df | polars with-column [(polars col a | polars contains ab | polars as b)] | polars collect;
                $df2.b",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "b".to_string(),
                            vec![
                                Value::test_bool(true),
                                Value::test_bool(false),
                                Value::test_bool(false),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Returns boolean indicating if pattern was found",
                example: "[abc acb acb] | polars into-df | polars contains ab",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "0".to_string(),
                            vec![
                                Value::test_bool(true),
                                Value::test_bool(false),
                                Value::test_bool(false),
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
                let df = lazy.collect(call.head)?;
                command_df(plugin, engine, call, df)
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
    let pattern: String = call.req(0)?;
    let res: NuExpression = expr
        .into_polars()
        .str()
        .contains(lit(pattern), false)
        .into();
    res.to_pipeline_data(plugin, engine, call.head)
}

fn command_df(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let pattern: String = call.req(0)?;

    let series = df.as_series(call.head)?;
    let chunked = series.str().map_err(|e| ShellError::GenericError {
        error: "The contains command only with string columns".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let res = chunked
        .contains(&pattern, false)
        .map_err(|e| ShellError::GenericError {
            error: "Error searching in series".into(),
            msg: e.to_string(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?;

    let df = NuDataFrame::try_from_series_vec(vec![res.into_series()], call.head)?;
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&Contains)
    }
}
