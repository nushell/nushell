use crate::{values::CustomValueSupport, PolarsPlugin};

use super::super::super::values::NuDataFrame;

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, SyntaxShape, Type,
};
use polars::prelude::{IntoSeries, StringMethods};

#[derive(Clone)]
pub struct AsDate;

impl PluginCommand for AsDate {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars as-date"
    }

    fn usage(&self) -> &str {
        r#"Converts string to date."#
    }

    fn extra_usage(&self) -> &str {
        r#"Format example:
        "%Y-%m-%d"    => 2021-12-31
        "%d-%m-%Y"    => 31-12-2021
        "%Y%m%d"      => 2021319 (2021-03-19)"#
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("format", SyntaxShape::String, "formatting date string")
            .switch("not-exact", "the format string may be contained in the date (e.g. foo-2021-01-01-bar could match 2021-01-01)", Some('n'))
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Converts string to date",
            example: r#"["2021-12-30" "2021-12-31"] | polars into-df | polars as-date "%Y-%m-%d""#,
            result: None,
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
    let format: String = call.req(0)?;
    let not_exact = call.has_flag("not-exact")?;

    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;
    let series = df.as_series(call.head)?;
    let casted = series.str().map_err(|e| ShellError::GenericError {
        error: "Error casting to string".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let res = if not_exact {
        casted.as_date_not_exact(Some(format.as_str()))
    } else {
        casted.as_date(Some(format.as_str()), false)
    };

    let mut res = res
        .map_err(|e| ShellError::GenericError {
            error: "Error creating datetime".into(),
            msg: e.to_string(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?
        .into_series();

    res.rename("date");

    let df = NuDataFrame::try_from_series_vec(vec![res], call.head)?;
    df.to_pipeline_data(plugin, engine, call.head)
}
