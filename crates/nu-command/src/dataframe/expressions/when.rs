use crate::dataframe::values::{Column, NuDataFrame, NuExpression, NuWhen};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use polars::prelude::when;

#[derive(Clone)]
pub struct ExprWhen;

impl Command for ExprWhen {
    fn name(&self) -> &str {
        "dfr when"
    }

    fn usage(&self) -> &str {
        "Creates and modifies a when expression"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "when expression",
                SyntaxShape::Any,
                "when expression used for matching",
            )
            .required(
                "then expression",
                SyntaxShape::Any,
                "expression that will be applied when predicate is true",
            )
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create a when conditions",
                example: "dfr when ((dfr col a) > 2) 4",
                result: None,
            },
            Example {
                description: "Create a when conditions",
                example: "dfr when ((dfr col a) > 2) 4 | dfr when ((dfr col a) < 0) 6",
                result: None,
            },
            Example {
                description: "Create a new column for the dataframe",
                example: r#"[[a b]; [6 2] [1 4] [4 1]]
   | dfr to-lazy
   | dfr with-column (
       dfr when ((dfr col a) > 2) 4 | dfr otherwise 5 | dfr as c
     )
   | dfr with-column (
       dfr when ((dfr col a) > 5) 10 | dfr when ((dfr col a) < 2) 6 | dfr otherwise 0 | dfr as d
     )
   | dfr collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new(
                            "a".to_string(),
                            vec![Value::Int(6), Value::Int(1), Value::Int(4)],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::Int(2), Value::Int(4), Value::Int(1)],
                        ),
                        Column::new(
                            "c".to_string(),
                            vec![Value::Int(4), Value::Int(5), Value::Int(4)],
                        ),
                        Column::new(
                            "d".to_string(),
                            vec![Value::Int(10), Value::Int(6), Value::Int(0)],
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
        let when_predicate: Value = call.req(engine_state, stack, 0)?;
        let when_predicate = NuExpression::try_from_value(when_predicate)?;

        let then_predicate: Value = call.req(engine_state, stack, 1)?;
        let then_predicate = NuExpression::try_from_value(then_predicate)?;

        let value = input.into_value(call.head);
        let when_then: NuWhen = match value {
            Value::Nothing => when(when_predicate.into_polars())
                .then(then_predicate.into_polars())
                .into(),
            v => match NuWhen::try_from_value(v)? {
                NuWhen::WhenThen(when_then) => when_then
                    .when(when_predicate.into_polars())
                    .then(then_predicate.into_polars())
                    .into(),
                NuWhen::WhenThenThen(when_then_then) => when_then_then
                    .when(when_predicate.into_polars())
                    .then(then_predicate.into_polars())
                    .into(),
            },
        };

        Ok(PipelineData::Value(when_then.into_value(call.head), None))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use crate::dataframe::eager::WithColumn;
    use crate::dataframe::expressions::otherwise::ExprOtherwise;
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
