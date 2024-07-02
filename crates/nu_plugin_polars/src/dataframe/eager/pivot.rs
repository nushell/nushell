use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

use polars_ops::pivot::pivot;

use crate::{
    dataframe::values::utils::convert_columns_string,
    values::{CustomValueSupport, PolarsPluginObject},
    Cacheable, PolarsPlugin,
};

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct PivotDF;

impl PluginCommand for PivotDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars pivot"
    }

    fn usage(&self) -> &str {
        "Pivot a DataFrame from wide to long format."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required_named(
                "on",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "column names for pivoting",
                Some('o'),
            )
            .required_named(
                "index",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "column names for indexes",
                Some('i'),
            )
            .required_named(
                "values",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "column names used as value columns",
                Some('v'),
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .switch(
                "streamable",
                "Whether or not to use the polars streaming engine. Only valid for lazy dataframes",
                Some('s'),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        match PolarsPluginObject::try_from_pipeline(plugin, input, call.head)? {
            PolarsPluginObject::NuDataFrame(df) => command_eager(plugin, engine, call, df),
            PolarsPluginObject::NuLazyFrame(lazy) => {
                command_eager(plugin, engine, call, lazy.collect(call.head)?)
            }
            _ => Err(ShellError::GenericError {
                error: "Must be a dataframe or lazy dataframe".into(),
                msg: "".into(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            }),
        }
        .map_err(LabeledError::from)
    }
}

fn command_eager(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let on_col: Vec<Value> = call.get_flag("on")?.expect("required value");
    let index_col: Vec<Value> = call.get_flag("index")?.expect("required value");
    let val_col: Vec<Value> = call.get_flag("values")?.expect("required value");

    let (on_col_string, id_col_span) = convert_columns_string(on_col, call.head)?;
    let (index_col_string, index_col_span) = convert_columns_string(index_col, call.head)?;
    let (val_col_string, val_col_span) = convert_columns_string(val_col, call.head)?;

    check_column_datatypes(df.as_ref(), &on_col_string, id_col_span)?;
    check_column_datatypes(df.as_ref(), &index_col_string, index_col_span)?;
    check_column_datatypes(df.as_ref(), &val_col_string, val_col_span)?;

    let polars_df = df.to_polars();
    // todo add other args
    let pivoted = pivot(
        &polars_df,
        &on_col_string,
        Some(&index_col_string),
        Some(&val_col_string),
        false,
        None,
        None,
    )
    .map_err(|e| ShellError::GenericError {
        error: format!("Pivot error: {e}"),
        msg: "".into(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let res = NuDataFrame::new(false, pivoted);
    res.to_pipeline_data(plugin, engine, call.head)
}

fn check_column_datatypes<T: AsRef<str>>(
    df: &polars::prelude::DataFrame,
    cols: &[T],
    col_span: Span,
) -> Result<(), ShellError> {
    if cols.is_empty() {
        return Err(ShellError::GenericError {
            error: "Merge error".into(),
            msg: "empty column list".into(),
            span: Some(col_span),
            help: None,
            inner: vec![],
        });
    }

    // Checking if they are same type
    if cols.len() > 1 {
        for w in cols.windows(2) {
            let l_series = df
                .column(w[0].as_ref())
                .map_err(|e| ShellError::GenericError {
                    error: "Error selecting columns".into(),
                    msg: e.to_string(),
                    span: Some(col_span),
                    help: None,
                    inner: vec![],
                })?;

            let r_series = df
                .column(w[1].as_ref())
                .map_err(|e| ShellError::GenericError {
                    error: "Error selecting columns".into(),
                    msg: e.to_string(),
                    span: Some(col_span),
                    help: None,
                    inner: vec![],
                })?;

            if l_series.dtype() != r_series.dtype() {
                return Err(ShellError::GenericError {
                    error: "Merge error".into(),
                    msg: "found different column types in list".into(),
                    span: Some(col_span),
                    help: Some(format!(
                        "datatypes {} and {} are incompatible",
                        l_series.dtype(),
                        r_series.dtype()
                    )),
                    inner: vec![],
                });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::test::test_polars_plugin_command;

    use super::*;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&PivotDF)
    }
}
