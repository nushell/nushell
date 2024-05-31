use crate::{values::CustomValueSupport, PolarsPlugin};

use super::super::super::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::prelude::{IntoSeries, StringNameSpaceImpl};

#[derive(Clone)]
pub struct Concatenate;

impl PluginCommand for Concatenate {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars concatenate"
    }

    fn usage(&self) -> &str {
        "Concatenates strings with other array."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "other",
                SyntaxShape::Any,
                "Other array with string to be concatenated",
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Concatenate string",
            example: r#"let other = ([za xs cd] | polars into-df);
    [abc abc abc] | polars into-df | polars concatenate $other"#,
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
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        command(plugin, engine, call, input).map_err(LabeledError::from)
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;

    let other: Value = call.req(0)?;
    let other_span = other.span();
    let other_df = NuDataFrame::try_from_value_coerce(plugin, &other, other_span)?;

    let other_series = other_df.as_series(other_span)?;
    let other_chunked = other_series.str().map_err(|e| ShellError::GenericError {
        error: "The concatenate only with string columns".into(),
        msg: e.to_string(),
        span: Some(other_span),
        help: None,
        inner: vec![],
    })?;

    let series = df.as_series(call.head)?;
    let chunked = series.str().map_err(|e| ShellError::GenericError {
        error: "The concatenate only with string columns".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let mut res = chunked.concat(other_chunked);

    res.rename(series.name());

    let df = NuDataFrame::try_from_series_vec(vec![res.into_series()], call.head)?;
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&Concatenate)
    }
}
