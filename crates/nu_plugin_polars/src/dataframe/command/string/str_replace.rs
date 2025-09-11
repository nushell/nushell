use crate::{
    PolarsPlugin, missing_flag_error,
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
pub struct StrReplace;

impl PluginCommand for StrReplace {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars str-replace"
    }

    fn description(&self) -> &str {
        "Replace the leftmost (sub)string by a regex pattern."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required_named(
                "pattern",
                SyntaxShape::String,
                "Regex pattern to be matched",
                Some('p'),
            )
            .required_named(
                "replace",
                SyntaxShape::String,
                "replacing string",
                Some('r'),
            )
            .input_output_types(vec![
                (
                    Type::Custom("dataframe".into()),
                    Type::Custom("dataframe".into()),
                ),
                (
                    Type::Custom("expression".into()),
                    Type::Custom("expression".into()),
                ),
            ])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Replaces string in column",
                example: "[[a]; [abc] [abcabc]] | polars into-df | polars select (polars col a | polars str-replace --pattern ab --replace AB) | polars collect",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "a".to_string(),
                            vec![Value::test_string("ABc"), Value::test_string("ABcabc")],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Replaces string",
                example: "[abc abc abc] | polars into-df | polars str-replace --pattern ab --replace AB",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "0".to_string(),
                            vec![
                                Value::test_string("ABc"),
                                Value::test_string("ABc"),
                                Value::test_string("ABc"),
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
    }
}

fn command_expr(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    expr: NuExpression,
) -> Result<PipelineData, ShellError> {
    let pattern: String = call
        .get_flag("pattern")?
        .ok_or_else(|| missing_flag_error("pattern", call.head))?;
    let pattern = lit(pattern);

    let replace: String = call
        .get_flag("replace")?
        .ok_or_else(|| missing_flag_error("replace", call.head))?;
    let replace = lit(replace);

    let res: NuExpression = expr
        .into_polars()
        .str()
        .replace(pattern, replace, false)
        .into();

    res.to_pipeline_data(plugin, engine, call.head)
}

fn command_df(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let pattern: String = call
        .get_flag("pattern")?
        .ok_or_else(|| missing_flag_error("pattern", call.head))?;
    let replace: String = call
        .get_flag("replace")?
        .ok_or_else(|| missing_flag_error("replace", call.head))?;

    let series = df.as_series(call.head)?;
    let chunked = series.str().map_err(|e| ShellError::GenericError {
        error: "Error conversion to string".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let mut res = chunked
        .replace(&pattern, &replace)
        .map_err(|e| ShellError::GenericError {
            error: "Error finding pattern other".into(),
            msg: e.to_string(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?;

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
        test_polars_plugin_command(&StrReplace)
    }
}
