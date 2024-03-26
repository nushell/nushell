use crate::dataframe::{
    utils::extract_strings,
    values::{Column, NuDataFrame, NuLazyFrame},
};
use nu_engine::command_prelude::*;

use polars::prelude::{IntoSeries, UniqueKeepStrategy};

#[derive(Clone)]
pub struct Unique;

impl Command for Unique {
    fn name(&self) -> &str {
        "dfr unique"
    }

    fn usage(&self) -> &str {
        "Returns unique values from a dataframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named(
                "subset",
                SyntaxShape::Any,
                "Subset of column(s) to use to maintain rows (lazy df)",
                Some('s'),
            )
            .switch(
                "last",
                "Keeps last unique value. Default keeps first value (lazy df)",
                Some('l'),
            )
            .switch(
                "maintain-order",
                "Keep the same order as the original DataFrame (lazy df)",
                Some('k'),
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe or lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Returns unique values from a series",
                example: "[2 2 2 2 2] | dfr into-df | dfr unique",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new("0".to_string(), vec![Value::test_int(2)])],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Creates a is unique expression from a column",
                example: "col a | unique",
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
    _engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let series = df.as_series(call.head)?;

    let res = series.unique().map_err(|e| ShellError::GenericError {
        error: "Error calculating unique values".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: Some("The str-slice command can only be used with string columns".into()),
        inner: vec![],
    })?;

    NuDataFrame::try_from_series(vec![res.into_series()], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

fn command_lazy(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let last = call.has_flag(engine_state, stack, "last")?;
    let maintain = call.has_flag(engine_state, stack, "maintain-order")?;

    let subset: Option<Value> = call.get_flag(engine_state, stack, "subset")?;
    let subset = match subset {
        Some(value) => Some(extract_strings(value)?),
        None => None,
    };

    let strategy = if last {
        UniqueKeepStrategy::Last
    } else {
        UniqueKeepStrategy::First
    };

    let lazy = lazy.into_polars();
    let lazy: NuLazyFrame = if maintain {
        lazy.unique(subset, strategy).into()
    } else {
        lazy.unique_stable(subset, strategy).into()
    };

    Ok(PipelineData::Value(lazy.into_value(call.head)?, None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(Unique {})])
    }
}
