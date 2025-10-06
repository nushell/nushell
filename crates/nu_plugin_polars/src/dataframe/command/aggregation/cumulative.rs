use crate::{PolarsPlugin, values::CustomValueSupport};

use crate::values::{
    Column, NuDataFrame, NuExpression, PolarsPluginObject, PolarsPluginType, cant_convert_err,
};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Value,
};
use polars::prelude::{DataType, IntoSeries, cum_max, cum_min, cum_sum};

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

    fn description(&self) -> &str {
        "Cumulative calculation for a column or series."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "type",
                SyntaxShape::String,
                "rolling operation. Values of min, max, and sum are accepted.",
            )
            .switch("reverse", "Reverse cumulative calculation", Some('r'))
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
                description: "Cumulative sum for a column",
                example: "[[a]; [1] [2] [3] [4] [5]]
                    | polars into-df
                    | polars select (polars col a | polars cumulative sum | polars as cum_a)
                    | polars collect",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "cum_a".to_string(),
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
            },
            Example {
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
            },
            Example {
                description: "Cumulative sum for a series in reverse order",
                example: "[1 2 3 4 5] | polars into-df | polars cumulative sum --reverse",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "0_cumulative_sum".to_string(),
                            vec![
                                Value::test_int(15),
                                Value::test_int(14),
                                Value::test_int(12),
                                Value::test_int(9),
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
        let value = input.into_value(call.head)?;
        let cum_type: Spanned<String> = call.req(0)?;
        let cum_type = CumulativeType::from_str(&cum_type.item, cum_type.span)?;
        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuDataFrame(df) => command_df(plugin, engine, call, cum_type, df),
            PolarsPluginObject::NuLazyFrame(lazy) => {
                command_df(plugin, engine, call, cum_type, lazy.collect(call.head)?)
            }
            PolarsPluginObject::NuExpression(expr) => {
                command_expr(plugin, engine, call, cum_type, expr)
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
    cum_type: CumulativeType,
    expr: NuExpression,
) -> Result<PipelineData, ShellError> {
    let reverse = call.has_flag("reverse")?;
    let polars = expr.into_polars();

    let res: NuExpression = match cum_type {
        CumulativeType::Max => polars.cum_max(reverse),
        CumulativeType::Min => polars.cum_min(reverse),
        CumulativeType::Sum => polars.cum_sum(reverse),
    }
    .into();

    res.to_pipeline_data(plugin, engine, call.head)
}

fn command_df(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    cum_type: CumulativeType,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let reverse = call.has_flag("reverse")?;
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
        test_polars_plugin_command(&Cumulative)
    }
}
