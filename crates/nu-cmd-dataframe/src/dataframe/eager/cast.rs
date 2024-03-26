use crate::dataframe::values::{str_to_dtype, NuDataFrame, NuExpression, NuLazyFrame};
use nu_engine::command_prelude::*;

use polars::prelude::*;

#[derive(Clone)]
pub struct CastDF;

impl Command for CastDF {
    fn name(&self) -> &str {
        "dfr cast"
    }

    fn usage(&self) -> &str {
        "Cast a column to a different dtype."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (
                    Type::Custom("expression".into()),
                    Type::Custom("expression".into()),
                ),
                (
                    Type::Custom("dataframe".into()),
                    Type::Custom("dataframe".into()),
                ),
            ])
            .required(
                "dtype",
                SyntaxShape::String,
                "The dtype to cast the column to",
            )
            .optional(
                "column",
                SyntaxShape::String,
                "The column to cast. Required when used with a dataframe.",
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Cast a column in a dataframe to a different dtype",
                example: "[[a b]; [1 2] [3 4]] | dfr into-df | dfr cast u8 a | dfr schema",
                result: Some(Value::record(
                    record! {
                        "a" => Value::string("u8", Span::test_data()),
                        "b" => Value::string("i64", Span::test_data()),
                    },
                    Span::test_data(),
                )),
            },
            Example {
                description: "Cast a column in a lazy dataframe to a different dtype",
                example: "[[a b]; [1 2] [3 4]] | dfr into-df | dfr into-lazy | dfr cast u8 a | dfr schema",
                result: Some(Value::record(
                    record! {
                        "a" => Value::string("u8", Span::test_data()),
                        "b" => Value::string("i64", Span::test_data()),
                    },
                    Span::test_data(),
                )),
            },
            Example {
                description: "Cast a column in a expression to a different dtype",
                example: r#"[[a b]; [1 2] [1 4]] | dfr into-df | dfr group-by a | dfr agg [ (dfr col b | dfr cast u8 | dfr min | dfr as "b_min") ] | dfr schema"#,
                result: None
            }
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
            let (dtype, column_nm) = df_args(engine_state, stack, call)?;
            let df = NuLazyFrame::try_from_value(value)?;
            command_lazy(call, column_nm, dtype, df)
        } else if NuDataFrame::can_downcast(&value) {
            let (dtype, column_nm) = df_args(engine_state, stack, call)?;
            let df = NuDataFrame::try_from_value(value)?;
            command_eager(call, column_nm, dtype, df)
        } else {
            let dtype: String = call.req(engine_state, stack, 0)?;
            let dtype = str_to_dtype(&dtype, call.head)?;

            let expr = NuExpression::try_from_value(value)?;
            let expr: NuExpression = expr.into_polars().cast(dtype).into();

            Ok(PipelineData::Value(
                NuExpression::into_value(expr, call.head),
                None,
            ))
        }
    }
}

fn df_args(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<(DataType, String), ShellError> {
    let dtype = dtype_arg(engine_state, stack, call)?;
    let column_nm: String =
        call.opt(engine_state, stack, 1)?
            .ok_or(ShellError::MissingParameter {
                param_name: "column_name".into(),
                span: call.head,
            })?;
    Ok((dtype, column_nm))
}

fn dtype_arg(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<DataType, ShellError> {
    let dtype: String = call.req(engine_state, stack, 0)?;
    str_to_dtype(&dtype, call.head)
}

fn command_lazy(
    call: &Call,
    column_nm: String,
    dtype: DataType,
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let column = col(&column_nm).cast(dtype);
    let lazy = lazy.into_polars().with_columns(&[column]);
    let lazy = NuLazyFrame::new(false, lazy);

    Ok(PipelineData::Value(
        NuLazyFrame::into_value(lazy, call.head)?,
        None,
    ))
}

fn command_eager(
    call: &Call,
    column_nm: String,
    dtype: DataType,
    nu_df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let mut df = nu_df.df;
    let column = df
        .column(&column_nm)
        .map_err(|e| ShellError::GenericError {
            error: format!("{e}"),
            msg: "".into(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?;

    let casted = column.cast(&dtype).map_err(|e| ShellError::GenericError {
        error: format!("{e}"),
        msg: "".into(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let _ = df
        .with_column(casted)
        .map_err(|e| ShellError::GenericError {
            error: format!("{e}"),
            msg: "".into(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?;

    let df = NuDataFrame::new(false, df);
    Ok(PipelineData::Value(df.into_value(call.head), None))
}

#[cfg(test)]
mod test {

    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(CastDF {})])
    }
}
