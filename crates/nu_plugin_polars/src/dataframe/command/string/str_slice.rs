use crate::{
    PolarsPlugin,
    values::{
        CustomValueSupport, NuExpression, PolarsPluginObject, PolarsPluginType, cant_convert_err,
    },
};

use super::super::super::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use polars::{
    prelude::{Expr, IntoSeries, NamedFrom, Null, StringNameSpaceImpl, lit},
    series::Series,
};

#[derive(Clone)]
pub struct StrSlice;

impl PluginCommand for StrSlice {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars str-slice"
    }

    fn description(&self) -> &str {
        "Slices the string from the start position until the selected length."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("start", SyntaxShape::Int, "start of slice")
            .named("length", SyntaxShape::Int, "optional length", Some('l'))
            .input_output_types(vec![
                (
                    PolarsPluginType::NuExpression.into(),
                    PolarsPluginType::NuExpression.into(),
                ),
                (
                    PolarsPluginType::NuDataFrame.into(),
                    PolarsPluginType::NuDataFrame.into(),
                ),
                (
                    PolarsPluginType::NuLazyFrame.into(),
                    PolarsPluginType::NuLazyFrame.into(),
                ),
            ])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Creates slices from the strings in a specified column",
                example: "[[a]; [abcded] [abc321] [abc123]] | polars into-df | polars select (polars col a | polars str-slice 1 --length 2) | polars collect",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "a".to_string(),
                            vec![
                                Value::test_string("bc"),
                                Value::test_string("bc"),
                                Value::test_string("bc"),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Creates slices from the strings",
                example: "[abcded abc321 abc123] | polars into-df | polars str-slice 1 --length 2",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "0".to_string(),
                            vec![
                                Value::test_string("bc"),
                                Value::test_string("bc"),
                                Value::test_string("bc"),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Creates slices from the strings without length",
                example: "[abcded abc321 abc123] | polars into-df | polars str-slice 1",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "0".to_string(),
                            vec![
                                Value::test_string("bcded"),
                                Value::test_string("bc321"),
                                Value::test_string("bc123"),
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
    let start = lit(call.req::<i64>(0)?);
    let length: Expr = call
        .get_flag::<i64>("length")?
        .map(lit)
        .unwrap_or(lit(Null {}));

    let res: NuExpression = expr.into_polars().str().slice(start, length).into();
    res.to_pipeline_data(plugin, engine, call.head)
}

fn command_df(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let start: i64 = call.req(0)?;
    let start = Series::new("".into(), &[start]).into();

    let length: Option<i64> = call.get_flag("length")?;
    let length = match length {
        Some(v) => Series::new("".into(), &[v as u64]),
        None => Series::new_null("".into(), 1),
    }
    .into();

    let series = df.as_series(call.head)?;

    let chunked = series.str().map_err(|e| ShellError::GenericError {
        error: "Error casting to string".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: Some("The str-slice command can only be used with string columns".into()),
        inner: vec![],
    })?;

    let res = chunked
        .str_slice(&start, &length)
        .map_err(|e| ShellError::GenericError {
            error: "Dataframe Error".into(),
            msg: e.to_string(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?
        .with_name(series.name().to_owned());

    let df = NuDataFrame::try_from_series_vec(vec![res.into_series()], call.head)?;
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&StrSlice)
    }
}
