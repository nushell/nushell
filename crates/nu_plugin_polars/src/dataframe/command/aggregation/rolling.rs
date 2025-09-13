use crate::values::{Column, NuDataFrame};
use crate::{PolarsPlugin, values::CustomValueSupport};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};
use polars::prelude::{DataType, IntoSeries, RollingOptionsFixedWindow, SeriesOpsTime};

enum RollType {
    Min,
    Max,
    Sum,
    Mean,
}

impl RollType {
    fn from_str(roll_type: &str, span: Span) -> Result<Self, ShellError> {
        match roll_type {
            "min" => Ok(Self::Min),
            "max" => Ok(Self::Max),
            "sum" => Ok(Self::Sum),
            "mean" => Ok(Self::Mean),
            _ => Err(ShellError::GenericError {
                error: "Wrong operation".into(),
                msg: "Operation not valid for cumulative".into(),
                span: Some(span),
                help: Some("Allowed values: min, max, sum, mean".into()),
                inner: vec![],
            }),
        }
    }

    fn to_str(&self) -> &'static str {
        match self {
            RollType::Min => "rolling_min",
            RollType::Max => "rolling_max",
            RollType::Sum => "rolling_sum",
            RollType::Mean => "rolling_mean",
        }
    }
}

#[derive(Clone)]
pub struct Rolling;

impl PluginCommand for Rolling {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars rolling"
    }

    fn description(&self) -> &str {
        "Rolling calculation for a series."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("type", SyntaxShape::String, "rolling operation")
            .required("window", SyntaxShape::Int, "Window size for rolling")
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Rolling sum for a series",
                example: "[1 2 3 4 5] | polars into-df | polars rolling sum 2 | polars drop-nulls",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "0_rolling_sum".to_string(),
                            vec![
                                Value::test_int(3),
                                Value::test_int(5),
                                Value::test_int(7),
                                Value::test_int(9),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Rolling max for a series",
                example: "[1 2 3 4 5] | polars into-df | polars rolling max 2 | polars drop-nulls",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "0_rolling_max".to_string(),
                            vec![
                                Value::test_int(2),
                                Value::test_int(3),
                                Value::test_int(4),
                                Value::test_int(5),
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
        command(plugin, engine, call, input)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let roll_type: Spanned<String> = call.req(0)?;
    let window_size: usize = call.req(1)?;

    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;
    let series = df.as_series(call.head)?;

    if let DataType::Object(..) = series.dtype() {
        return Err(ShellError::GenericError {
            error: "Found object series".into(),
            msg: "Series of type object cannot be used for rolling operation".into(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        });
    }

    let roll_type = RollType::from_str(&roll_type.item, roll_type.span)?;

    let rolling_opts = RollingOptionsFixedWindow {
        window_size,
        min_periods: window_size,
        ..RollingOptionsFixedWindow::default()
    };

    let res = match roll_type {
        RollType::Max => series.rolling_max(rolling_opts),
        RollType::Min => series.rolling_min(rolling_opts),
        RollType::Sum => series.rolling_sum(rolling_opts),
        RollType::Mean => series.rolling_mean(rolling_opts),
    };

    let mut res = res.map_err(|e| ShellError::GenericError {
        error: "Error calculating rolling values".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let name = format!("{}_{}", series.name(), roll_type.to_str());
    res.rename(name.into());

    let df = NuDataFrame::try_from_series_vec(vec![res.into_series()], call.head)?;
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&Rolling)
    }
}
