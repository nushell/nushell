use crate::dataframe::values::{Column, NuDataFrame, NuExpression, NuWhen};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct ExprOtherwise;

impl Command for ExprOtherwise {
    fn name(&self) -> &str {
        "otherwise"
    }

    fn usage(&self) -> &str {
        "completes a when expression"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "otherwise expression",
                SyntaxShape::Any,
                "expressioini to apply when no when predicate matches",
            )
            .input_type(Type::Any)
            .output_type(Type::Custom("expression".into()))
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create a when conditions",
                example: "when ((col a) > 2) 4 | otherwise 5",
                result: None,
            },
            Example {
                description: "Create a when conditions",
                example: "when ((col a) > 2) 4 | when ((col a) < 0) 6 | otherwise 0",
                result: None,
            },
            Example {
                description: "Create a new column for the dataframe",
                example: r#"[[a b]; [6 2] [1 4] [4 1]]
   | into lazy
   | with-column (
       when ((col a) > 2) 4 | otherwise 5 | as c
     )
   | with-column (
       when ((col a) > 5) 10 | when ((col a) < 2) 6 | otherwise 0 | as d
     )
   | collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(vec![
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
                    ])
                    .expect("simple df for test should not fail")
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
        let otherwise_predicate: Value = call.req(engine_state, stack, 0)?;
        let otherwise_predicate = NuExpression::try_from_value(otherwise_predicate)?;

        let value = input.into_value(call.head);
        let complete: NuExpression = match NuWhen::try_from_value(value)? {
            NuWhen::WhenThen(when_then) => when_then
                .otherwise(otherwise_predicate.into_polars())
                .into(),
            NuWhen::WhenThenThen(when_then_then) => when_then_then
                .otherwise(otherwise_predicate.into_polars())
                .into(),
        };

        Ok(PipelineData::Value(complete.into_value(call.head), None))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use crate::dataframe::eager::WithColumn;
    use crate::dataframe::expressions::when::ExprWhen;
    use crate::dataframe::expressions::{ExprAlias, ExprAsNu, ExprCol};

    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![
            Box::new(WithColumn {}),
            Box::new(ExprCol {}),
            Box::new(ExprAlias {}),
            Box::new(ExprWhen {}),
            Box::new(ExprOtherwise {}),
            Box::new(ExprAsNu {}),
        ])
    }
}
