use crate::{
    PolarsPlugin,
    dataframe::values::{NuExpression, NuLazyFrame},
    values::{CustomValueSupport, PolarsPluginObject, PolarsPluginType, cant_convert_err},
};

use crate::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

use polars_plan::prelude::lit;

#[derive(Clone)]
pub struct Shift;

impl PluginCommand for Shift {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars shift"
    }

    fn description(&self) -> &str {
        "Shifts the values by a given period."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("period", SyntaxShape::Int, "shift period")
            .named(
                "fill",
                SyntaxShape::Any,
                "Expression used to fill the null values (lazy df)",
                Some('f'),
            )
            .input_output_types(vec![
                (
                    Type::Custom("dataframe".into()),
                    Type::Custom("dataframe".into()),
                ),
                (
                    Type::Custom("expression".into()),
                    Type::Custom("expression".into()),
                ),
            ])
            .category(Category::Custom("dataframe or lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Shifts the values by a given period",
                example: "[1 2 2 3 3] | polars into-df | polars shift 2 | polars drop-nulls",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "0".to_string(),
                            vec![Value::test_int(1), Value::test_int(2), Value::test_int(2)],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Shifts the values by a given period, fill absent values with 0",
                example: "[1 2 2 3 3] | polars into-lazy | polars shift 2 --fill 0 | polars collect",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "0".to_string(),
                            vec![
                                Value::test_int(0),
                                Value::test_int(0),
                                Value::test_int(1),
                                Value::test_int(2),
                                Value::test_int(2),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Shift values of a column, fill absent values with 0",
                example: "[[a]; [1] [2] [2] [3] [3]]
                    | polars into-lazy
                    | polars with-column {b: (polars col a | polars shift 2 --fill 0)}
                    | polars collect",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![
                                    Value::test_int(1),
                                    Value::test_int(2),
                                    Value::test_int(2),
                                    Value::test_int(3),
                                    Value::test_int(3),
                                ],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![
                                    Value::test_int(0),
                                    Value::test_int(0),
                                    Value::test_int(1),
                                    Value::test_int(2),
                                    Value::test_int(2),
                                ],
                            ),
                        ],
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

        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuDataFrame(df) => command_eager(plugin, engine, call, df),
            PolarsPluginObject::NuLazyFrame(lazy) => command_lazy(plugin, engine, call, lazy),
            PolarsPluginObject::NuExpression(expr) => {
                let shift: i64 = call.req(0)?;
                let fill: Option<Value> = call.get_flag("fill")?;

                let res: NuExpression = match fill {
                    Some(ref fill) => {
                        let fill_expr = NuExpression::try_from_value(plugin, fill)?.into_polars();
                        expr.into_polars()
                            .shift_and_fill(lit(shift), fill_expr)
                            .into()
                    }
                    None => expr.into_polars().shift(lit(shift)).into(),
                };

                res.to_pipeline_data(plugin, engine, call.head)
            }
            _ => Err(cant_convert_err(
                &value,
                &[
                    PolarsPluginType::NuDataFrame,
                    PolarsPluginType::NuLazyGroupBy,
                    PolarsPluginType::NuExpression,
                ],
            )),
        }
        .map_err(LabeledError::from)
        .map(|pd| pd.set_metadata(metadata))
    }
}

fn command_eager(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let period: i64 = call.req(0)?;
    let series = df.as_series(call.head)?.shift(period);

    let df = NuDataFrame::try_from_series_vec(vec![series], call.head)?;
    df.to_pipeline_data(plugin, engine, call.head)
}

fn command_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let shift: i64 = call.req(0)?;
    let fill: Option<Value> = call.get_flag("fill")?;

    let lazy = lazy.to_polars();

    let lazy: NuLazyFrame = match fill {
        Some(ref fill) => {
            let expr = NuExpression::try_from_value(plugin, fill)?.into_polars();
            lazy.shift_and_fill(lit(shift), expr).into()
        }
        None => lazy.shift(shift).into(),
    };

    lazy.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&Shift)
    }
}
