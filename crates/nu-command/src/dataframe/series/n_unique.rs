use super::super::values::{Column, NuDataFrame};
use crate::dataframe::values::NuExpression;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Value,
};

#[derive(Clone)]
pub struct NUnique;

impl Command for NUnique {
    fn name(&self) -> &str {
        "dfr count-unique"
    }

    fn usage(&self) -> &str {
        "Counts unique values or creates a n-unique expression"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Counts unique values",
                example: "[1 1 2 2 3 3 4] | dfr to-df | dfr count-unique",
                result: Some(
                    NuDataFrame::try_from_columns(vec![Column::new(
                        "count_unique".to_string(),
                        vec![Value::test_int(4)],
                    )])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Creates a is n-unique expression from a column",
                example: "dfr col a | dfr n-unique",
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

        if NuExpression::can_downcast(&value) {
            let expr = NuExpression::try_from_value(value)?;
            let expr: NuExpression = expr.into_polars().n_unique().into();

            Ok(PipelineData::Value(
                NuExpression::into_value(expr, call.head),
                None,
            ))
        } else if NuDataFrame::can_downcast(&value) {
            let df = NuDataFrame::try_from_value(value)?;
            command(engine_state, stack, call, df)
        } else {
            Err(ShellError::CantConvert(
                "expression or query".into(),
                value.get_type().to_string(),
                value.span()?,
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
    let res = df.as_series(call.head)?.n_unique().map_err(|e| {
        ShellError::GenericError(
            "Error counting unique values".into(),
            e.to_string(),
            Some(call.head),
            None,
            Vec::new(),
        )
    })?;

    let value = Value::Int {
        val: res as i64,
        span: call.head,
    };

    NuDataFrame::try_from_columns(vec![Column::new("count_unique".to_string(), vec![value])])
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(NUnique {})])
    }
}
