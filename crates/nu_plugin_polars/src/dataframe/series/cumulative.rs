use crate::{values::CustomValueSupport, PolarsPlugin};

use super::super::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};
use polars::prelude::{DataType, IntoSeries};
use polars_ops::prelude::{cum_max, cum_min, cum_sum};

enum CumulativeType {
    Min,
    Max,
    Sum,
}

impl CumulativeType {
    fn from_str(roll_type: &str, span: Span) -> Result<Self, ShellError> {
        match roll_type {
            "min" => Ok(Self::Min),
            "max" => Ok(Self::Max),
            "sum" => Ok(Self::Sum),
            _ => Err(ShellError::GenericError {
                error: "Wrong operation".into(),
                msg: "Operation not valid for cumulative".into(),
                span: Some(span),
                help: Some("Allowed values: max, min, sum".into()),
                inner: vec![],
            }),
        }
    }

    fn to_str(&self) -> &'static str {
        match self {
            CumulativeType::Min => "cumulative_min",
            CumulativeType::Max => "cumulative_max",
            CumulativeType::Sum => "cumulative_sum",
        }
    }
}

#[derive(Clone)]
pub struct Cumulative;

impl PluginCommand for Cumulative {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars cumulative"
    }

    fn usage(&self) -> &str {
        "Cumulative calculation for a series."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("type", SyntaxShape::String, "rolling operation")
            .switch("reverse", "Reverse cumulative calculation", Some('r'))
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Cumulative sum for a series",
            example: "[1 2 3 4 5] | polars into-df | polars cumulative sum",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "0_cumulative_sum".to_string(),
                        vec![
                            Value::test_int(1),
                            Value::test_int(3),
                            Value::test_int(6),
                            Value::test_int(10),
                            Value::test_int(15),
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
    let cum_type: Spanned<String> = call.req(0)?;
    let reverse = call.has_flag("reverse")?;

    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;
    let series = df.as_series(call.head)?;

    if let DataType::Object(..) = series.dtype() {
        return Err(ShellError::GenericError {
            error: "Found object series".into(),
            msg: "Series of type object cannot be used for cumulative operation".into(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        });
    }

    let cum_type = CumulativeType::from_str(&cum_type.item, cum_type.span)?;
    let mut res = match cum_type {
        CumulativeType::Max => cum_max(&series, reverse),
        CumulativeType::Min => cum_min(&series, reverse),
        CumulativeType::Sum => cum_sum(&series, reverse),
    }
    .map_err(|e| ShellError::GenericError {
        error: "Error creating cumulative".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let name = format!("{}_{}", series.name(), cum_type.to_str());
    res.rename(&name);

    let df = NuDataFrame::try_from_series_vec(vec![res.into_series()], call.head)?;
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&Cumulative)
    }
}
