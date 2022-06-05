use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Value,
};

use crate::dataframe::values::utils::convert_columns_string;

use super::super::values::{Column, NuDataFrame};

#[derive(Clone)]
pub struct MeltDF;

impl Command for MeltDF {
    fn name(&self) -> &str {
        "dfr melt"
    }

    fn usage(&self) -> &str {
        "Unpivot a DataFrame from wide to long format"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required_named(
                "columns",
                SyntaxShape::Table,
                "column names for melting",
                Some('c'),
            )
            .required_named(
                "values",
                SyntaxShape::Table,
                "column names used as value columns",
                Some('v'),
            )
            .named(
                "variable-name",
                SyntaxShape::String,
                "optional name for variable column",
                Some('r'),
            )
            .named(
                "value-name",
                SyntaxShape::String,
                "optional name for value column",
                Some('l'),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "melt dataframe",
            example:
                "[[a b c d]; [x 1 4 a] [y 2 5 b] [z 3 6 c]] | dfr to-df | dfr melt -c [b c] -v [a d]",
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "b".to_string(),
                        vec![
                            Value::Int(1),
                            Value::Int(2),
                            Value::Int(3),
                            Value::Int(1),
                            Value::Int(2),
                            Value::Int(3),
                        ],
                    ),
                    Column::new(
                        "c".to_string(),
                        vec![
                            Value::Int(4),
                            Value::Int(5),
                            Value::Int(6),
                            Value::Int(4),
                            Value::Int(5),
                            Value::Int(6),
                        ],
                    ),
                    Column::new(
                        "variable".to_string(),
                        vec![
                            Value::String("a".into()),
                            Value::String("a".into()),
                            Value::String("a".into()),
                            Value::String("d".into()),
                            Value::String("d".into()),
                            Value::String("d".into()),
                        ],
                    ),
                    Column::new(
                        "value".to_string(),
                        vec![
                            Value::String("x".into()),
                            Value::String("y".into()),
                            Value::String("z".into()),
                            Value::String("a".into()),
                            Value::String("b".into()),
                            Value::String("c".into()),
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
        command(engine_state, stack, call, input)
    }
}

fn command(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let id_col: Vec<Value> = call
        .get_flag(engine_state, stack, "columns")?
        .expect("required value");
    let val_col: Vec<Value> = call
        .get_flag(engine_state, stack, "values")?
        .expect("required value");

    let value_name: Option<Spanned<String>> = call.get_flag(engine_state, stack, "value-name")?;
    let variable_name: Option<Spanned<String>> =
        call.get_flag(engine_state, stack, "variable-name")?;

    let (id_col_string, id_col_span) = convert_columns_string(id_col, call.head)?;
    let (val_col_string, val_col_span) = convert_columns_string(val_col, call.head)?;

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    check_column_datatypes(df.as_ref(), &id_col_string, id_col_span)?;
    check_column_datatypes(df.as_ref(), &val_col_string, val_col_span)?;

    let mut res = df
        .as_ref()
        .melt(&id_col_string, &val_col_string)
        .map_err(|e| {
            ShellError::GenericError(
                "Error calculating melt".into(),
                e.to_string(),
                Some(call.head),
                None,
                Vec::new(),
            )
        })?;

    if let Some(name) = &variable_name {
        res.rename("variable", &name.item).map_err(|e| {
            ShellError::GenericError(
                "Error renaming column".into(),
                e.to_string(),
                Some(name.span),
                None,
                Vec::new(),
            )
        })?;
    }

    if let Some(name) = &value_name {
        res.rename("value", &name.item).map_err(|e| {
            ShellError::GenericError(
                "Error renaming column".into(),
                e.to_string(),
                Some(name.span),
                None,
                Vec::new(),
            )
        })?;
    }

    Ok(PipelineData::Value(
        NuDataFrame::dataframe_into_value(res, call.head),
        None,
    ))
}

fn check_column_datatypes<T: AsRef<str>>(
    df: &polars::prelude::DataFrame,
    cols: &[T],
    col_span: Span,
) -> Result<(), ShellError> {
    if cols.is_empty() {
        return Err(ShellError::GenericError(
            "Merge error".into(),
            "empty column list".into(),
            Some(col_span),
            None,
            Vec::new(),
        ));
    }

    // Checking if they are same type
    if cols.len() > 1 {
        for w in cols.windows(2) {
            let l_series = df.column(w[0].as_ref()).map_err(|e| {
                ShellError::GenericError(
                    "Error selecting columns".into(),
                    e.to_string(),
                    Some(col_span),
                    None,
                    Vec::new(),
                )
            })?;

            let r_series = df.column(w[1].as_ref()).map_err(|e| {
                ShellError::GenericError(
                    "Error selecting columns".into(),
                    e.to_string(),
                    Some(col_span),
                    None,
                    Vec::new(),
                )
            })?;

            if l_series.dtype() != r_series.dtype() {
                return Err(ShellError::GenericError(
                    "Merge error".into(),
                    "found different column types in list".into(),
                    Some(col_span),
                    Some(format!(
                        "datatypes {} and {} are incompatible",
                        l_series.dtype(),
                        r_series.dtype()
                    )),
                    Vec::new(),
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(MeltDF {})])
    }
}
