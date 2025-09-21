use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type};
use polars::prelude::PlSmallStr;

use crate::{
    PolarsPlugin,
    values::{CustomValueSupport, NuDataFrame},
};

pub struct QCutSeries;

impl PluginCommand for QCutSeries {
    type Plugin = PolarsPlugin;
    fn name(&self) -> &str {
        "polars qcut"
    }

    fn description(&self) -> &str {
        "Bin continuous values into discrete categories based on their quantiles for a series."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build(self.name())
            .required("quantiles", SyntaxShape::Any, "Either a list of quantile probabilities between 0 and 1 or a positive integer determining the number of bins with uniform probability.")
            .named(
                "labels",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "Names of the categories. The number of labels must be equal to the number of cut points plus one.",
                Some('l'),
            )
            .switch("left_closed", "Set the intervals to be left-closed instead of right-closed.", Some('c'))
            .switch("include_breaks", "Include a column with the right endpoint of the bin each observation falls in. This will change the data type of the output from a Categorical to a Struct.", Some('b'))
            .switch("allow_duplicates", "If set, duplicates in the resulting quantiles are dropped, rather than raising an error. This can happen even with unique probabilities, depending on the data.", Some('d'))
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Divide a column into three categories according to pre-defined quantile probabilities.",
            example: r#"[-2, -1, 0, 1, 2] | polars into-df | polars qcut [0.25, 0.75] --labels ["a", "b", "c"]"#,
            result: None,
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, nu_protocol::LabeledError> {
        let metadata = input.metadata();
        command(plugin, engine, call, input)
            .map_err(|e| e.into())
            .map(|pd| pd.set_metadata(metadata))
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;
    let series = df.as_series(call.head)?;

    let quantiles = call.req::<Vec<f64>>(0)?;

    let labels: Option<Vec<PlSmallStr>> = call.get_flag::<Vec<String>>("labels")?.map(|l| {
        l.into_iter()
            .map(PlSmallStr::from)
            .collect::<Vec<PlSmallStr>>()
    });

    let left_closed = call.has_flag("left_closed")?;
    let include_breaks = call.has_flag("include_breaks")?;
    let allow_duplicates = call.has_flag("allow_duplicates")?;

    let new_series = polars_ops::series::qcut(
        &series,
        quantiles,
        labels,
        left_closed,
        allow_duplicates,
        include_breaks,
    )
    .map_err(|e| ShellError::GenericError {
        error: "Error cutting series".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    NuDataFrame::try_from_series(new_series, call.head)?.to_pipeline_data(plugin, engine, call.head)
}
