use crate::values::conversion::convert_duration;
use crate::values::{
    Column, NuDataFrame, NuExpression, NuLazyFrame, NuLazyGroupBy, PolarsPluginObject,
    PolarsPluginType, cant_convert_err,
};
use crate::{PolarsPlugin, values::CustomValueSupport};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Value, shell_error::generic::GenericError,
};
use polars::prelude::{DataType, IntoSeries, RollingOptionsFixedWindow, SeriesOpsTime};
use polars::time::{ClosedWindow, Duration as PolarsDuration, RollingGroupOptions};
use polars_lazy::dsl::col;

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
                GenericError::new("Wrong operation", "Operation not valid for rolling", span)
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
        "Rolling calculation for a series or expression, or a rolling group-by for a lazyframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .optional(
                "type",
                SyntaxShape::String,
                "Rolling operation (min, max, sum, mean). Required for dataframe and expression inputs.",
            )
            .optional(
                "window",
                SyntaxShape::Int,
                "Window size for rolling. Required for dataframe and expression inputs.",
            )
            .named(
                "index-column",
                SyntaxShape::String,
                "Time or index column to roll over. Required for lazyframe input.",
                Some('i'),
            )
            .named(
                "period",
                SyntaxShape::OneOf(vec![SyntaxShape::Duration, SyntaxShape::String, SyntaxShape::Int]),
                "Rolling window width (e.g. 7day, \"7d\", 1wk, \"1mo\", \"2i\", ). Required for lazyframe input.",
                Some('p'),
            )
            .named(
                "offset",
                SyntaxShape::OneOf(vec![SyntaxShape::Duration, SyntaxShape::String, SyntaxShape::Int]),
                "Offset of the rolling window relative to the index column (default: 1ns).",
                Some('o'),
            )
            .named(
                "closed",
                SyntaxShape::String,
                "Which boundary of the window is closed: left, right, both, none (default: left).",
                Some('c'),
            )
            .named(
                "group-by",
                SyntaxShape::List(Box::new(SyntaxShape::Any)),
                "Additional expressions to group by within each rolling window.",
                Some('g'),
            )
            .input_output_types(vec![
                (
                    PolarsPluginType::NuDataFrame.into(),
                    PolarsPluginType::NuDataFrame.into(),
                ),
                (
                    PolarsPluginType::NuExpression.into(),
                    PolarsPluginType::NuExpression.into(),
                ),
                (
                    PolarsPluginType::NuLazyFrame.into(),
                    PolarsPluginType::NuLazyGroupBy.into(),
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
                description: "Rolling sum for an expression in a lazyframe",
                example: "[[a]; [1] [2] [3] [4] [5]]
    | polars into-df
    | polars select (polars col a | polars rolling sum 2 | polars as roll_a)
    | polars collect
    | polars drop-nulls
    | polars collect",
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
            Example {
                description: "Rolling sum over a 2-row integer window on a lazyframe",
                example: r#"[[idx val]; [1 10] [2 20] [3 30] [4 40] [5 50]]
    | polars into-lazy
    | polars rolling --index-column idx --period 2i
    | polars agg [(polars col val | polars sum | polars as val_sum)]
    | polars collect
    | polars sort-by idx
    | polars collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "idx".to_string(),
                                vec![
                                    Value::test_int(1),
                                    Value::test_int(2),
                                    Value::test_int(3),
                                    Value::test_int(4),
                                    Value::test_int(5),
                                ],
                            ),
                            Column::new(
                                "val_sum".to_string(),
                                vec![
                                    Value::test_int(50),
                                    Value::test_int(70),
                                    Value::test_int(90),
                                    Value::test_int(50),
                                    Value::test_int(0),
                                ],
                            ),
                        ],
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

        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuLazyFrame(lazy) => command_lazy(plugin, engine, call, lazy),
            other => {
                let roll_type: Spanned<String> = call.req(0)?;
                let roll_type = RollType::from_str(&roll_type.item, roll_type.span)
                    .map_err(LabeledError::from)?;
                match other {
                    PolarsPluginObject::NuDataFrame(df) => {
                        command_eager(plugin, engine, call, roll_type, df)
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
            }
        }
        .map_err(LabeledError::from)
        .map(|pd| pd.set_metadata(metadata))
    }
}

fn command_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    mut lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let index_column: String = call.get_flag("index-column")?.ok_or_else(|| {
        ShellError::Generic(GenericError::new(
            "Missing required flag",
            "--index-column is required for lazyframe input",
            call.head,
        ))
    })?;

    let period: PolarsDuration = call
        .get_flag("period")?
        .map(convert_duration)
        .transpose()?
        .unwrap_or_else(|| PolarsDuration::new(1));

    let offset: PolarsDuration = call
        .get_flag("offset")?
        .map(convert_duration)
        .transpose()?
        .unwrap_or_else(|| PolarsDuration::new(1));

    let closed_window = match call.get_flag_value("closed") {
        Some(Value::String { val, .. }) => match val.as_str() {
            "left" => ClosedWindow::Left,
            "right" => ClosedWindow::Right,
            "both" => ClosedWindow::Both,
            "none" => ClosedWindow::None,
            other => {
                return Err(ShellError::Generic(GenericError::new(
                    "Invalid closed window value",
                    format!("Got '{other}', expected one of: left, right, both, none"),
                    call.head,
                )));
            }
        },
        _ => ClosedWindow::Left,
    };

    let group_by_exprs = match call.get_flag_value("group-by") {
        Some(list) => NuExpression::extract_exprs(plugin, list)?,
        None => vec![],
    };

    let options = RollingGroupOptions {
        index_column: index_column.as_str().into(),
        period,
        offset,
        closed_window,
    };

    let schema = lazy.schema().clone()?;
    let from_eager = lazy.from_eager;
    let group_by = lazy
        .to_polars()
        .rolling(col(index_column.as_str()), group_by_exprs, options);
    NuLazyGroupBy::new(group_by, from_eager, schema).to_pipeline_data(plugin, engine, call.head)
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
