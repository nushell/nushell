use super::super::values::NuExpression;

use chrono::{FixedOffset, DateTime, Utc};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct ExprDatePart;

impl Command for ExprDatePart {
    fn name(&self) -> &str {
        "dfr datepart"
    }

    fn usage(&self) -> &str {
        "Creates an expression for capturing the specified datepart in a column."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "Datepart name",
                SyntaxShape::String,
                "Part of the date to capture.  Possible values are year, quarter, month, week, weekday, day, hour, minute, second, millisecond, microsecond, nanosecond",
            )
            .input_type(Type::Custom("expression".into()))
            .output_type(Type::Custom("expression".into()))
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        let example_value: DateTime<FixedOffset> = Utc::now().into();
        vec![Example {
            description: "Creates an expression to capture date parts",
            example: "dfr col a | dfr datepart year | dfr into-nu",
            result: {
                let cols = vec!["expr".into(), "value".into()];
                let expr = Value::test_string("column");
                let value = Value::test_string("a");
                let expr = Value::Record {
                    cols,
                    vals: vec![expr, value],
                    span: Span::test_data(),
                };

                let cols = vec!["expr".into(), "datepart".into()];
                let value = Value::test_date(example_value);

                let record = Value::Record {
                    cols,
                    vals: vec![expr, value],
                    span: Span::test_data(),
                };

                Some(record)
            },
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["datepart", "date", "year", "month", "week", "weekday", "quarter", "day", "hour", "minute", "second", "millisecond", "microsecond", "nanosecond"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let part: String = call.req(engine_state, stack, 0)?;

        let expr = NuExpression::try_from_pipeline(input, call.head)?;
        let expr_dt = expr.into_polars().dt();
        let expr = match part.as_str() {
            "year" => expr_dt.year(),
            "quarter" => expr_dt.quarter(),
            "month" => expr_dt.month(),
            "week" => expr_dt.week(),
            "day" => expr_dt.day(),
            "hour" => expr_dt.hour(),
            "minute" => expr_dt.minute(),
            "second" => expr_dt.second(),
            "millisecond" => expr_dt.millisecond(),
            "microsecond" => expr_dt.microsecond(),
            "nanosecond" => expr_dt.nanosecond(),
            _ => {
                return Err(ShellError::UnsupportedInput(
                    format!("{} is not a valid datepart, expected one of year, month, day, hour, minute, second, millisecond, microsecond, nanosecond", part),
                    "value originates from here".to_string(),
                    call.head,
                    call.head, // need to do better here
                ));
            }            
        }.into();

        Ok(PipelineData::Value(
            NuExpression::into_value(expr, call.head),
            None,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;
    use crate::dataframe::expressions::ExprAsNu;
    use crate::dataframe::expressions::ExprCol;

    #[test]
    fn test_examples() {
        test_dataframe(vec![
            Box::new(ExprDatePart {}),
            Box::new(ExprCol {}),
            Box::new(ExprAsNu {}),
        ])
    }
}
