use crate::dataframe::values::{Column, NuDataFrame, NuExpression};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct LazyFillNA;

impl Command for LazyFillNA {
    fn name(&self) -> &str {
        "dfr fill-nan"
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
                example: "[1 2 NaN 3 NaN] | dfr into-df | dfr fill-nan 0",
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
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Fills the NaN values of a whole dataframe",
                example: "[[a b]; [0.2 1] [0.1 NaN]] | dfr into-df | dfr fill-nan 0",
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
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let fill: Value = call.req(engine_state, stack, 0)?;
        let value = input.into_value(call.head);

        if NuExpression::can_downcast(&value) {
            let expr = NuExpression::try_from_value(value)?;
            let fill = NuExpression::try_from_value(fill)?.into_polars();
            let expr: NuExpression = expr.into_polars().fill_nan(fill).into();

            Ok(PipelineData::Value(
                NuExpression::into_value(expr, call.head),
                None,
            ))
        } else {
            let val_span = value.span();
            let frame = NuDataFrame::try_from_value(value)?;
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
                NuDataFrame::try_from_columns(dataframe, None)?.into_value(call.head),
                None,
            ))
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(LazyFillNA {})])
    }
}
