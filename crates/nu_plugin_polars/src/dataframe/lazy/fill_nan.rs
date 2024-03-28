use crate::{
    dataframe::values::{Column, NuDataFrame, NuExpression},
    values::PhysicalType,
    Cacheable, CustomValueSupport, PolarsPlugin,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

#[derive(Clone)]
pub struct LazyFillNA;

impl PluginCommand for LazyFillNA {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars fill-nan"
    }

    fn usage(&self) -> &str {
        "Replaces NaN values with the given expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "fill",
                SyntaxShape::Any,
                "Expression to use to fill the NAN values",
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Fills the NaN values with 0",
                example: "[1 2 NaN 3 NaN] | polars into-df | polars fill-nan 0",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "0".to_string(),
                            vec![
                                Value::test_int(1),
                                Value::test_int(2),
                                Value::test_int(0),
                                Value::test_int(3),
                                Value::test_int(0),
                            ],
                        )],
                        None,
                    )
                    .expect("Df for test should not fail")
                    .base_value(Span::test_data())
                    .expect("rendering base value should not fail"),
                ),
            },
            Example {
                description: "Fills the NaN values of a whole dataframe",
                example: "[[a b]; [0.2 1] [0.1 NaN]] | polars into-df | polars fill-nan 0",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_float(0.2), Value::test_float(0.1)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(1), Value::test_int(0)],
                            ),
                        ],
                        None,
                    )
                    .expect("Df for test should not fail")
                    .base_value(Span::test_data())
                    .expect("rendering base value should not fail"),
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
        let fill: Value = call.req(0)?;
        let value = input.into_value(call.head);

        match PhysicalType::try_from_value(plugin, &value)? {
            PhysicalType::NuDataFrame(df) => {
                Ok(cmd_df(plugin, engine, call, df, fill, value.span())?)
            }
            PhysicalType::NuLazyFrame(lazy) => Ok(cmd_df(
                plugin,
                engine,
                call,
                lazy.collect(value.span())?,
                fill,
                value.span(),
            )?),
            PhysicalType::NuExpression(expr) => Ok(cmd_expr(plugin, engine, call, expr, fill)?),
            _ => Err(LabeledError::new("dataframe or expression is required")
                .with_label("Invalid Type", call.head)),
        }
    }
}

fn cmd_df(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    frame: NuDataFrame,
    fill: Value,
    val_span: Span,
) -> Result<PipelineData, ShellError> {
    let columns = frame.columns(val_span)?;
    let dataframe = columns
        .into_iter()
        .map(|column| {
            let column_name = column.name().to_string();
            let values = column
                .into_iter()
                .map(|value| {
                    let span = value.span();
                    match value {
                        Value::Float { val, .. } => {
                            if val.is_nan() {
                                fill.clone()
                            } else {
                                value
                            }
                        }
                        Value::List { vals, .. } => {
                            NuDataFrame::fill_list_nan(vals, span, fill.clone())
                        }
                        _ => value,
                    }
                })
                .collect::<Vec<Value>>();
            Column::new(column_name, values)
        })
        .collect::<Vec<Column>>();
    Ok(PipelineData::Value(
        NuDataFrame::try_from_columns(dataframe, None)?
            .cache(plugin, engine)?
            .into_value(call.head),
        None,
    ))
}

fn cmd_expr(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    expr: NuExpression,
    fill: Value,
) -> Result<PipelineData, ShellError> {
    let fill = NuExpression::try_from_value(plugin, &fill)?.into_polars();
    let expr: NuExpression = expr.into_polars().fill_nan(fill).into();

    Ok(PipelineData::Value(
        expr.cache(plugin, engine)?.into_value(call.head),
        None,
    ))
}

// todo: fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![Box::new(LazyFillNA {})])
//     }
// }
