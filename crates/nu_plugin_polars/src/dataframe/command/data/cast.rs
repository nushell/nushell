use crate::{
    PolarsPlugin,
    dataframe::values::{NuExpression, NuLazyFrame, str_to_dtype},
    values::{CustomValueSupport, PolarsPluginObject, PolarsPluginType, cant_convert_err},
};

use crate::values::NuDataFrame;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value, record,
};
use polars::prelude::*;

#[derive(Clone)]
pub struct CastDF;

impl PluginCommand for CastDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars cast"
    }

    fn description(&self) -> &str {
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

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Cast a column in a dataframe to a different dtype",
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars cast u8 a | polars schema",
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
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars into-lazy | polars cast u8 a | polars schema",
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
                example: r#"[[a b]; [1 2] [1 4]] | polars into-df | polars group-by a | polars agg [ (polars col b | polars cast u8 | polars min | polars as "b_min") ] | polars schema"#,
                result: None,
            },
        ]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let value = input.into_value(call.head)?;
        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuLazyFrame(lazy) => {
                let (dtype, column_nm) = df_args(call)?;
                command_lazy(plugin, engine, call, column_nm, dtype, lazy)
            }
            PolarsPluginObject::NuDataFrame(df) => {
                let (dtype, column_nm) = df_args(call)?;
                command_eager(plugin, engine, call, column_nm, dtype, df)
            }
            PolarsPluginObject::NuExpression(expr) => {
                let dtype: String = call.req(0)?;
                let dtype = str_to_dtype(&dtype, call.head)?;
                let expr: NuExpression = expr.into_polars().cast(dtype).into();
                expr.to_pipeline_data(plugin, engine, call.head)
            }
            _ => Err(cant_convert_err(
                &value,
                &[
                    PolarsPluginType::NuDataFrame,
                    PolarsPluginType::NuLazyFrame,
                    PolarsPluginType::NuExpression,
                ],
            )),
        }
        .map_err(LabeledError::from)
        .map(|pd| pd.set_metadata(metadata))
    }
}

fn df_args(call: &EvaluatedCall) -> Result<(DataType, String), ShellError> {
    let dtype = dtype_arg(call)?;
    let column_nm: String = call.opt(1)?.ok_or(ShellError::MissingParameter {
        param_name: "column_name".into(),
        span: call.head,
    })?;
    Ok((dtype, column_nm))
}

fn dtype_arg(call: &EvaluatedCall) -> Result<DataType, ShellError> {
    let dtype: String = call.req(0)?;
    str_to_dtype(&dtype, call.head)
}

fn command_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    column_nm: String,
    dtype: DataType,
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let column = col(&column_nm).cast(dtype);
    let lazy = lazy.to_polars().with_columns(&[column]);
    let lazy = NuLazyFrame::new(false, lazy);
    lazy.to_pipeline_data(plugin, engine, call.head)
}

fn command_eager(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    column_nm: String,
    dtype: DataType,
    nu_df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let mut df = (*nu_df.df).clone();
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
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&CastDF)
    }
}
