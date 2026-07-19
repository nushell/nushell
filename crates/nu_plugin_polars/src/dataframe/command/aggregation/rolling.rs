use crate::values::{
    Column, NuDataFrame, NuExpression, PolarsPluginObject, PolarsPluginType, cant_convert_err,
};
use crate::{PolarsPlugin, values::CustomValueSupport};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Value, shell_error::generic::GenericError,
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
            _ => Err(ShellError::Generic(
                GenericError::new(
                    "Wrong operation",
                    "Operation not valid for cumulative",
                    span,
                )
                .with_help("Allowed values: min, max, sum, mean"),
            )),
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
            .required("type", SyntaxShape::String, "Rolling operation.")
            .required("window", SyntaxShape::Int, "Window size for rolling.")
            .input_output_types(vec![
                (
                    PolarsPluginType::NuDataFrame.into(),
                    PolarsPluginType::NuDataFrame.into(),
                ),
                (
                    PolarsPluginType::NuLazyFrame.into(),
                    PolarsPluginType::NuLazyFrame.into(),
                ),
                (
                    PolarsPluginType::NuExpression.into(),
                    PolarsPluginType::NuExpression.into(),
                ),
            ])
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
                        Span::test_data(),
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
                        Span::test_data(),
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Rolling sum for an expression in a lazy frame",
                example: "[[a]; [1] [2] [3] [4] [5]]
                    | polars into-df
                    | polars select (polars col a | polars rolling sum 2 | polars as roll_a)
                    | polars collect
                    | polars drop-nulls",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "roll_a".to_string(),
                            vec![
                                Value::test_int(3),
                                Value::test_int(5),
                                Value::test_int(7),
                                Value::test_int(9),
                            ],
                        )],
                        None,
                        Span::test_data(),
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
        mut input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.take_metadata();
        let value = input.into_value(call.head)?;
        let roll_type: Spanned<String> = call.req(0)?;
        let roll_type = RollType::from_str(&roll_type.item, roll_type.span)
            .map_err(LabeledError::from)?;

        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuDataFrame(df) => {
                command_eager(plugin, engine, call, roll_type, df)
            }
            PolarsPluginObject::NuLazyFrame(lazy) => {
                command_eager(plugin, engine, call, roll_type, lazy.collect(call.head)?)
            }
            PolarsPluginObject::NuExpression(expr) => {
                command_expr(plugin, engine, call, roll_type, expr)
            }
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
    roll_type: RollType,
    expr: NuExpression,
) -> Result<PipelineData, ShellError> {
    let window_size: usize = call.req(1)?;
    let rolling_opts = RollingOptionsFixedWindow {
        window_size,
        min_periods: window_size,
        ..RollingOptionsFixedWindow::default()
    };

    let polars_expr = expr.into_polars();
    let res: NuExpression = match roll_type {
        RollType::Max => polars_expr.rolling_max(rolling_opts),
        RollType::Min => polars_expr.rolling_min(rolling_opts),
        RollType::Sum => polars_expr.rolling_sum(rolling_opts),
        RollType::Mean => polars_expr.rolling_mean(rolling_opts),
    }
    .into();

    res.to_pipeline_data(plugin, engine, call.head)
}

fn command_eager(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    roll_type: RollType,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let window_size: usize = call.req(1)?;

    let series = df.as_series(call.head)?;

    if let DataType::Object(..) = series.dtype() {
        return Err(ShellError::Generic(GenericError::new(
            "Found object series",
            "Series of type object cannot be used for rolling operation",
            call.head,
        )));
    }

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

    let mut res = res.map_err(|e| {
        ShellError::Generic(GenericError::new(
            "Error calculating rolling values",
            e.to_string(),
            call.head,
        ))
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
