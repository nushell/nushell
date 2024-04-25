use crate::dataframe::values::{Column, NuDataFrame, NuExpression, NuWhen};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct ExprOtherwise;

impl Command for ExprOtherwise {
    fn name(&self) -> &str {
        "dfr otherwise"
    }

    fn usage(&self) -> &str {
        "Completes a when expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "otherwise expression",
                SyntaxShape::Any,
                "expression to apply when no when predicate matches",
            )
            .input_output_type(Type::Any, Type::Custom("expression".into()))
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create a when conditions",
                example: "dfr when ((dfr col a) > 2) 4 | dfr otherwise 5",
                result: None,
            },
            Example {
                description: "Create a when conditions",
                example:
                    "dfr when ((dfr col a) > 2) 4 | dfr when ((dfr col a) < 0) 6 | dfr otherwise 0",
                result: None,
            },
            Example {
                description: "Create a new column for the dataframe",
                example: r#"[[a b]; [6 2] [1 4] [4 1]]
   | dfr into-lazy
   | dfr with-column (
    dfr when ((dfr col a) > 2) 4 | dfr otherwise 5 | dfr as c
     )
   | dfr with-column (
    dfr when ((dfr col a) > 5) 10 | dfr when ((dfr col a) < 2) 6 | dfr otherwise 0 | dfr as d
     )
   | dfr collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(6), Value::test_int(1), Value::test_int(4)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(2), Value::test_int(4), Value::test_int(1)],
                            ),
                            Column::new(
                                "c".to_string(),
                                vec![Value::test_int(4), Value::test_int(5), Value::test_int(4)],
                            ),
                            Column::new(
                                "d".to_string(),
                                vec![Value::test_int(10), Value::test_int(6), Value::test_int(0)],
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

    fn search_terms(&self) -> Vec<&str> {
        vec!["condition", "else"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let otherwise_predicate: Value = call.req(engine_state, stack, 0)?;
        let otherwise_predicate = NuExpression::try_from_value(otherwise_predicate)?;

        let value = input.into_value(call.head);
        let complete: NuExpression = match NuWhen::try_from_value(value)? {
            NuWhen::Then(then) => then.otherwise(otherwise_predicate.into_polars()).into(),
            NuWhen::ChainedThen(chained_when) => chained_when
                .otherwise(otherwise_predicate.into_polars())
                .into(),
        };

        Ok(PipelineData::Value(complete.into_value(call.head), None))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use crate::dataframe::eager::{ToNu, WithColumn};
    use crate::dataframe::expressions::when::ExprWhen;
    use crate::dataframe::expressions::{ExprAlias, ExprCol};

    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![
            Box::new(WithColumn {}),
            Box::new(ExprCol {}),
            Box::new(ExprAlias {}),
            Box::new(ExprWhen {}),
            Box::new(ExprOtherwise {}),
            Box::new(ToNu {}),
        ])
    }
}
