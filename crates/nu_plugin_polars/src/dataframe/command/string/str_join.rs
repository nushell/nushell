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
use polars::{prelude::StringNameSpaceImpl, series::IntoSeries};

#[derive(Clone)]
pub struct StrJoin;

impl PluginCommand for StrJoin {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars str-join"
    }

    fn description(&self) -> &str {
        "Concatenates strings within a column or dataframes"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .optional("other", SyntaxShape::Any, "Other dataframe with a single series of strings to be concatenated. Required when used with a dataframe, ignored when used as an expression.")
            .named("delimiter", SyntaxShape::String, "Delimiter to join strings within an expression. Other dataframe when used with a dataframe.", Some('d'))
            .switch("ignore-nulls", "Ignore null values. Only available when used as an expression.", Some('n'))
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
                description: "Join strings in a column",
                example: r#"[[a]; [abc] [abc] [abc]] | polars into-df | polars select (polars col a | polars str-join -d ',') | polars collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "a".to_string(),
                            vec![Value::test_string("abc,abc,abc")],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "StrJoin strings across two series",
                example: r#"let other = ([za xs cd] | polars into-df);
    [abc abc abc] | polars into-df | polars str-join $other"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "0".to_string(),
                            vec![
                                Value::test_string("abcza"),
                                Value::test_string("abcxs"),
                                Value::test_string("abccd"),
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
            _ => Err(cant_convert_err(&value, &[PolarsPluginType::NuExpression])),
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
    let delimiter = call
        .get_flag::<String>("delimiter")?
        .map(|x| x.to_string())
        .unwrap_or_else(|| "".to_string());
    let ignore_nulls = call.has_flag("ignore-nulls")?;
    let res: NuExpression = expr
        .into_polars()
        .str()
        .join(&delimiter, ignore_nulls)
        .into();

    res.to_pipeline_data(plugin, engine, call.head)
}

fn command_df(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let other: Value = call.req(0).map_err(|_| ShellError::MissingParameter {
        param_name: "other".into(),
        span: call.head,
    })?;
    let other_span = other.span();
    let other_df = NuDataFrame::try_from_value_coerce(plugin, &other, other_span)?;

    let other_series = other_df.as_series(other_span)?;
    let other_chunked = other_series.str().map_err(|e| ShellError::GenericError {
        error: "The str-join command only works with string columns".into(),
        msg: e.to_string(),
        span: Some(other_span),
        help: None,
        inner: vec![],
    })?;

    let series = df.as_series(call.head)?;
    let chunked = series.str().map_err(|e| ShellError::GenericError {
        error: "The str-join command only works only with string columns".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let mut res = chunked.concat(other_chunked);

    res.rename(series.name().to_owned());

    let df = NuDataFrame::try_from_series_vec(vec![res.into_series()], call.head)?;
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&StrJoin)
    }
}
