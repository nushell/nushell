use crate::dataframe::values::{NuExpression, NuLazyFrame};

use super::super::values::{Column, NuDataFrame};

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Shift;

impl Command for Shift {
    fn name(&self) -> &str {
        "shift"
    }

    fn usage(&self) -> &str {
        "Shifts the values by a given period"
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
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe or lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Shifts the values by a given period",
            example: "[1 2 2 3 3] | into df | shift 2 | drop-nulls",
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "0".to_string(),
                    vec![Value::test_int(1), Value::test_int(2), Value::test_int(2)],
                )])
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
        let value = input.into_value(call.head);

        if NuLazyFrame::can_downcast(&value) {
            let df = NuLazyFrame::try_from_value(value)?;
            command_lazy(engine_state, stack, call, df)
        } else {
            let df = NuDataFrame::try_from_value(value)?;
            command_eager(engine_state, stack, call, df)
        }
    }
}

fn command_eager(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let period: i64 = call.req(engine_state, stack, 0)?;
    let series = df.as_series(call.head)?.shift(period);

    NuDataFrame::try_from_series(vec![series], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

fn command_lazy(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let shift: i64 = call.req(engine_state, stack, 0)?;
    let fill: Option<Value> = call.get_flag(engine_state, stack, "fill")?;

    let lazy = lazy.into_polars();

    let lazy: NuLazyFrame = match fill {
        Some(fill) => {
            let expr = NuExpression::try_from_value(fill)?.into_polars();
            lazy.shift_and_fill(shift, expr).into()
        }
        None => lazy.shift(shift).into(),
    };

    Ok(PipelineData::Value(lazy.into_value(call.head)?, None))
}

#[cfg(test)]
mod test {
    use super::super::super::eager::DropNulls;
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(Shift {}), Box::new(DropNulls {})])
    }
}
