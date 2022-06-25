use super::super::values::{Column, NuDataFrame};
use crate::dataframe::values::{NuExpression, NuLazyFrame};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct WithColumn;

impl Command for WithColumn {
    fn name(&self) -> &str {
        "with-column"
    }

    fn usage(&self) -> &str {
        "Adds a series to the dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named("name", SyntaxShape::String, "new column name", Some('n'))
            .rest(
                "series or expressions",
                SyntaxShape::Any,
                "series to be added or expressions used to define the new columns",
            )
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe or lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Adds a series to the dataframe",
                example: r#"[[a b]; [1 2] [3 4]]
    | into df
    | with-column ([5 6] | into df) --name c"#,
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new(
                            "a".to_string(),
                            vec![Value::test_int(1), Value::test_int(3)],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::test_int(2), Value::test_int(4)],
                        ),
                        Column::new(
                            "c".to_string(),
                            vec![Value::test_int(5), Value::test_int(6)],
                        ),
                    ])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Adds a series to the dataframe",
                example: r#"[[a b]; [1 2] [3 4]]
    | into lazy
    | with-column [
        ((col a) * 2 | as "c")
        ((col a) * 3 | as "d")
      ]
    | collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new(
                            "a".to_string(),
                            vec![Value::test_int(1), Value::test_int(3)],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::test_int(2), Value::test_int(4)],
                        ),
                        Column::new(
                            "c".to_string(),
                            vec![Value::test_int(2), Value::test_int(6)],
                        ),
                        Column::new(
                            "d".to_string(),
                            vec![Value::test_int(3), Value::test_int(9)],
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
        let value = input.into_value(call.head);

        if NuLazyFrame::can_downcast(&value) {
            let df = NuLazyFrame::try_from_value(value)?;
            command_lazy(engine_state, stack, call, df)
        } else if NuDataFrame::can_downcast(&value) {
            let df = NuDataFrame::try_from_value(value)?;
            command_eager(engine_state, stack, call, df)
        } else {
            Err(ShellError::CantConvert(
                "lazy or eager dataframe".into(),
                value.get_type().to_string(),
                value.span()?,
                None,
            ))
        }
    }
}

fn command_eager(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    mut df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let new_column: Value = call.req(engine_state, stack, 0)?;
    let column_span = new_column.span()?;

    if NuExpression::can_downcast(&new_column) {
        let vals: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let value = Value::List {
            vals,
            span: call.head,
        };
        let expressions = NuExpression::extract_exprs(value)?;
        let lazy = NuLazyFrame::new(true, df.lazy().with_columns(&expressions));

        let df = lazy.collect(call.head)?;

        Ok(PipelineData::Value(df.into_value(call.head), None))
    } else {
        let mut other = NuDataFrame::try_from_value(new_column)?.as_series(column_span)?;

        let name = match call.get_flag::<String>(engine_state, stack, "name")? {
            Some(name) => name,
            None => other.name().to_string(),
        };

        let series = other.rename(&name).clone();

        df.as_mut()
            .with_column(series)
            .map_err(|e| {
                ShellError::GenericError(
                    "Error adding column to dataframe".into(),
                    e.to_string(),
                    Some(column_span),
                    None,
                    Vec::new(),
                )
            })
            .map(|df| {
                PipelineData::Value(
                    NuDataFrame::dataframe_into_value(df.clone(), call.head),
                    None,
                )
            })
    }
}

fn command_lazy(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let vals: Vec<Value> = call.rest(engine_state, stack, 0)?;
    let value = Value::List {
        vals,
        span: call.head,
    };
    let expressions = NuExpression::extract_exprs(value)?;

    let lazy: NuLazyFrame = lazy.into_polars().with_columns(&expressions).into();

    Ok(PipelineData::Value(
        NuLazyFrame::into_value(lazy, call.head)?,
        None,
    ))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;
    use crate::dataframe::expressions::ExprAlias;
    use crate::dataframe::expressions::ExprCol;

    #[test]
    fn test_examples() {
        test_dataframe(vec![
            Box::new(WithColumn {}),
            Box::new(ExprAlias {}),
            Box::new(ExprCol {}),
        ])
    }
}
