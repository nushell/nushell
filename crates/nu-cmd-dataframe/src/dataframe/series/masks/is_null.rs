use crate::dataframe::values::{Column, NuDataFrame, NuExpression};
use nu_engine::command_prelude::*;
use polars::prelude::IntoSeries;

#[derive(Clone)]
pub struct IsNull;

impl Command for IsNull {
    fn name(&self) -> &str {
        "dfr is-null"
    }

    fn usage(&self) -> &str {
        "Creates mask where value is null."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (
                    Type::Custom("expression".into()),
                    Type::Custom("expression".into()),
                ),
                (
                    Type::Custom("dataframe".into()),
                    Type::Custom("dataframe".into()),
                ),
            ])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create mask where values are null",
                example: r#"let s = ([5 6 0 8] | dfr into-df);
    let res = ($s / $s);
    $res | dfr is-null"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "is_null".to_string(),
                            vec![
                                Value::test_bool(false),
                                Value::test_bool(false),
                                Value::test_bool(true),
                                Value::test_bool(false),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Creates a is null expression from a column",
                example: "dfr col a | dfr is-null",
                result: None,
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
        let value = input.into_value(call.head);
        if NuDataFrame::can_downcast(&value) {
            let df = NuDataFrame::try_from_value(value)?;
            command(engine_state, stack, call, df)
        } else {
            let expr = NuExpression::try_from_value(value)?;
            let expr: NuExpression = expr.into_polars().is_null().into();

            Ok(PipelineData::Value(
                NuExpression::into_value(expr, call.head),
                None,
            ))
        }
    }
}

fn command(
    _engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let mut res = df.as_series(call.head)?.is_null();
    res.rename("is_null");

    NuDataFrame::try_from_series(vec![res.into_series()], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::dataframe::lazy::aggregate::LazyAggregate;
    use crate::dataframe::lazy::groupby::ToLazyGroupBy;
    use crate::dataframe::test_dataframe::{build_test_engine_state, test_dataframe_example};

    #[test]
    fn test_examples_dataframe() {
        let mut engine_state = build_test_engine_state(vec![Box::new(IsNull {})]);
        test_dataframe_example(&mut engine_state, &IsNull.examples()[0]);
    }

    #[test]
    fn test_examples_expression() {
        let mut engine_state = build_test_engine_state(vec![
            Box::new(IsNull {}),
            Box::new(LazyAggregate {}),
            Box::new(ToLazyGroupBy {}),
        ]);
        test_dataframe_example(&mut engine_state, &IsNull.examples()[1]);
    }
}
