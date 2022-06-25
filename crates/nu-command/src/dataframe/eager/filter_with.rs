use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use polars::prelude::LazyFrame;

use crate::dataframe::values::{NuExpression, NuLazyFrame};

use super::super::values::{Column, NuDataFrame};

#[derive(Clone)]
pub struct FilterWith;

impl Command for FilterWith {
    fn name(&self) -> &str {
        "filter-with"
    }

    fn usage(&self) -> &str {
        "Filters dataframe using a mask or expression as reference"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "mask or expression",
                SyntaxShape::Any,
                "boolean mask used to filter data",
            )
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe or lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Filter dataframe using a bool mask",
                example: r#"let mask = ([true false] | into df);
    [[a b]; [1 2] [3 4]] | into df | filter-with $mask"#,
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new("a".to_string(), vec![Value::test_int(1)]),
                        Column::new("b".to_string(), vec![Value::test_int(2)]),
                    ])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Filter dataframe using an expression",
                example: "[[a b]; [1 2] [3 4]] | into df | filter-with ((col a) > 1)",
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new("a".to_string(), vec![Value::test_int(3)]),
                        Column::new("b".to_string(), vec![Value::test_int(4)]),
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
    let mask_value: Value = call.req(engine_state, stack, 0)?;
    let mask_span = mask_value.span()?;

    if NuExpression::can_downcast(&mask_value) {
        let expression = NuExpression::try_from_value(mask_value)?;
        let lazy = NuLazyFrame::new(true, df.lazy());
        let lazy = lazy.apply_with_expr(expression, LazyFrame::filter);

        Ok(PipelineData::Value(
            NuLazyFrame::into_value(lazy, call.head)?,
            None,
        ))
    } else {
        let mask = NuDataFrame::try_from_value(mask_value)?.as_series(mask_span)?;
        let mask = mask.bool().map_err(|e| {
            ShellError::GenericError(
                "Error casting to bool".into(),
                e.to_string(),
                Some(mask_span),
                Some("Perhaps you want to use a series with booleans as mask".into()),
                Vec::new(),
            )
        })?;

        df.as_ref()
            .filter(mask)
            .map_err(|e| {
                ShellError::GenericError(
                    "Error filtering dataframe".into(),
                    e.to_string(),
                    Some(call.head),
                    Some("The only allowed column types for dummies are String or Int".into()),
                    Vec::new(),
                )
            })
            .map(|df| PipelineData::Value(NuDataFrame::dataframe_into_value(df, call.head), None))
    }
}

fn command_lazy(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let expr: Value = call.req(engine_state, stack, 0)?;
    let expr = NuExpression::try_from_value(expr)?;

    let lazy = lazy.apply_with_expr(expr, LazyFrame::filter);

    Ok(PipelineData::Value(
        NuLazyFrame::into_value(lazy, call.head)?,
        None,
    ))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;
    use crate::dataframe::expressions::ExprCol;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(FilterWith {}), Box::new(ExprCol {})])
    }
}
