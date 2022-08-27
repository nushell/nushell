use crate::dataframe::values::{Column, NuDataFrame, NuExpression};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use polars::prelude::{lit, DataType};

#[derive(Clone)]
pub struct ExprIsIn;

impl Command for ExprIsIn {
    fn name(&self) -> &str {
        "is-in"
    }

    fn usage(&self) -> &str {
        "Creates an is-in expression"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "list",
                SyntaxShape::List(Box::new(SyntaxShape::Any)),
                "List to check if values are in",
            )
            .input_type(Type::Custom("expression".into()))
            .output_type(Type::Custom("expression".into()))
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates a is-in expression",
            example: r#"let df = ([[a b]; [one 1] [two 2] [three 3]] | into df);
    $df | with-column (col a | is-in [one two] | as a_in)"#,
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "a".to_string(),
                        vec![
                            Value::test_string("one"),
                            Value::test_string("two"),
                            Value::test_string("three"),
                        ],
                    ),
                    Column::new(
                        "b".to_string(),
                        vec![Value::test_int(1), Value::test_int(2), Value::test_int(3)],
                    ),
                    Column::new(
                        "a_in".to_string(),
                        vec![
                            Value::test_bool(true),
                            Value::test_bool(true),
                            Value::test_bool(false),
                        ],
                    ),
                ])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let list: Vec<Value> = call.req(engine_state, stack, 0)?;
        let expr = NuExpression::try_from_pipeline(input, call.head)?;

        let values = NuDataFrame::try_from_columns(vec![Column::new("list".to_string(), list)])?;
        let list = values.as_series(call.head)?;

        if matches!(list.dtype(), DataType::Object(..)) {
            return Err(ShellError::IncompatibleParametersSingle(
                "Cannot use a mixed list as argument".into(),
                call.head,
            ));
        }

        let expr: NuExpression = expr.into_polars().is_in(lit(list)).into();
        Ok(PipelineData::Value(expr.into_value(call.head), None))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;
    use crate::dataframe::eager::WithColumn;
    use crate::dataframe::expressions::alias::ExprAlias;
    use crate::dataframe::expressions::col::ExprCol;

    #[test]
    fn test_examples() {
        test_dataframe(vec![
            Box::new(ExprIsIn {}),
            Box::new(ExprAlias {}),
            Box::new(ExprCol {}),
            Box::new(WithColumn {}),
        ])
    }
}
