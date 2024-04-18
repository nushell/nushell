use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Spanned, SyntaxShape,
    Type,
};
use polars::prelude::NamedFrom;
use polars::series::Series;

use crate::{values::CustomValueSupport, PolarsPlugin};

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct SampleDF;

impl PluginCommand for SampleDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars sample"
    }

    fn usage(&self) -> &str {
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
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Sample rows from dataframe",
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars sample --n-rows 1",
                result: None, // No expected value because sampling is random
            },
            Example {
                description: "Shows sample row using fraction and replace",
                example:
                    "[[a b]; [1 2] [3 4] [5 6]] | polars into-df | polars sample --fraction 0.5 --replace",
                result: None, // No expected value because sampling is random
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
            .sample_n(&Series::new("s", &[rows.item]), replace, shuffle, seed)
            .map_err(|e| ShellError::GenericError {
                error: "Error creating sample".into(),
                msg: e.to_string(),
                span: Some(rows.span),
                help: None,
                inner: vec![],
            }),
        (None, Some(frac)) => df
            .as_ref()
            .sample_frac(&Series::new("frac", &[frac.item]), replace, shuffle, seed)
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
