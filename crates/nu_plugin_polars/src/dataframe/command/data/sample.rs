use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Value,
};
use polars::prelude::NamedFrom;
use polars::series::Series;

use crate::{PolarsPlugin, values::CustomValueSupport};

use crate::values::{Column, NuDataFrame, PolarsPluginType};

#[derive(Clone)]
pub struct SampleDF;

impl PluginCommand for SampleDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars sample"
    }

    fn description(&self) -> &str {
        "Create sample dataframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named(
                "n-rows",
                SyntaxShape::Int,
                "number of rows to be taken from dataframe",
                Some('n'),
            )
            .named(
                "fraction",
                SyntaxShape::Number,
                "fraction of dataframe to be taken",
                Some('f'),
            )
            .named(
                "seed",
                SyntaxShape::Number,
                "seed for the selection",
                Some('s'),
            )
            .switch("replace", "sample with replace", Some('e'))
            .switch("shuffle", "shuffle sample", Some('u'))
            .input_output_types(vec![
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
                description: "Sample rows from dataframe",
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars sample --n-rows 1",
                result: None, // No expected value because sampling is random
            },
            Example {
                description: "Shows sample row using fraction and replace",
                example: "[[a b]; [1 2] [3 4] [5 6]] | polars into-df | polars sample --fraction 0.5 --replace",
                result: None, // No expected value because sampling is random
            },
            Example {
                description: "Shows sample row using using predefined seed 1",
                example: "[[a b]; [1 2] [3 4] [5 6]] | polars into-df | polars sample --seed 1 --n-rows 1",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new("a".to_string(), vec![Value::test_int(5)]),
                            Column::new("b".to_string(), vec![Value::test_int(6)]),
                        ],
                        None,
                    )
                    .expect("should not fail")
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
        command(plugin, engine, call, input).map_err(LabeledError::from)
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let rows: Option<Spanned<i64>> = call.get_flag("n-rows")?;
    let fraction: Option<Spanned<f64>> = call.get_flag("fraction")?;
    let seed: Option<u64> = call.get_flag::<i64>("seed")?.map(|val| val as u64);
    let replace: bool = call.has_flag("replace")?;
    let shuffle: bool = call.has_flag("shuffle")?;

    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;

    let df = match (rows, fraction) {
        (Some(rows), None) => df
            .as_ref()
            .sample_n(
                &Series::new("s".into(), &[rows.item]),
                replace,
                shuffle,
                seed,
            )
            .map_err(|e| ShellError::GenericError {
                error: "Error creating sample".into(),
                msg: e.to_string(),
                span: Some(rows.span),
                help: None,
                inner: vec![],
            }),
        (None, Some(frac)) => df
            .as_ref()
            .sample_frac(
                &Series::new("frac".into(), &[frac.item]),
                replace,
                shuffle,
                seed,
            )
            .map_err(|e| ShellError::GenericError {
                error: "Error creating sample".into(),
                msg: e.to_string(),
                span: Some(frac.span),
                help: None,
                inner: vec![],
            }),
        (Some(_), Some(_)) => Err(ShellError::GenericError {
            error: "Incompatible flags".into(),
            msg: "Only one selection criterion allowed".into(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        }),
        (None, None) => Err(ShellError::GenericError {
            error: "No selection".into(),
            msg: "No selection criterion was found".into(),
            span: Some(call.head),
            help: Some("Perhaps you want to use the flag -n or -f".into()),
            inner: vec![],
        }),
    };
    let df = NuDataFrame::new(false, df?);
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&SampleDF)
    }
}
