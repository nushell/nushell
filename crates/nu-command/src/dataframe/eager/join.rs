use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Value,
};
use polars::prelude::JoinType;

use crate::dataframe::values::utils::convert_columns_string;

use super::super::values::{Column, NuDataFrame};

#[derive(Clone)]
pub struct JoinDF;

impl Command for JoinDF {
    fn name(&self) -> &str {
        "dfr join"
    }

    fn usage(&self) -> &str {
        "Joins a dataframe using columns as reference"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("dataframe", SyntaxShape::Any, "right dataframe to join")
            .required_named(
                "left",
                SyntaxShape::Table,
                "left column names to perform join",
                Some('l'),
            )
            .required_named(
                "right",
                SyntaxShape::Table,
                "right column names to perform join",
                Some('r'),
            )
            .named(
                "type",
                SyntaxShape::String,
                "type of join. Inner by default",
                Some('t'),
            )
            .named(
                "suffix",
                SyntaxShape::String,
                "suffix for the columns of the right dataframe",
                Some('s'),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "inner join dataframe",
            example: r#"let right = ([[a b c]; [1 2 5] [3 4 5] [5 6 6]] | dfr to-df);
    $right | dfr join $right -l [a b] -r [a b]"#,
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "a".to_string(),
                        vec![Value::test_int(1), Value::test_int(3), Value::test_int(5)],
                    ),
                    Column::new(
                        "b".to_string(),
                        vec![Value::test_int(2), Value::test_int(4), Value::test_int(6)],
                    ),
                    Column::new(
                        "c".to_string(),
                        vec![Value::test_int(5), Value::test_int(5), Value::test_int(6)],
                    ),
                    Column::new(
                        "c_right".to_string(),
                        vec![Value::test_int(5), Value::test_int(5), Value::test_int(6)],
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
    let r_df: Value = call.req(engine_state, stack, 0)?;
    let l_col: Vec<Value> = call
        .get_flag(engine_state, stack, "left")?
        .expect("required value in syntax");
    let r_col: Vec<Value> = call
        .get_flag(engine_state, stack, "right")?
        .expect("required value in syntax");
    let suffix: Option<String> = call.get_flag(engine_state, stack, "suffix")?;
    let join_type_op: Option<Spanned<String>> = call.get_flag(engine_state, stack, "type")?;

    let join_type = match join_type_op {
        None => JoinType::Inner,
        Some(val) => match val.item.as_ref() {
            "inner" => JoinType::Inner,
            "outer" => JoinType::Outer,
            "left" => JoinType::Left,
            _ => {
                return Err(ShellError::SpannedLabeledErrorHelp(
                    "Incorrect join type".into(),
                    "Invalid join type".into(),
                    val.span,
                    "Options: inner, outer or left".into(),
                ))
            }
        },
    };

    let (l_col_string, l_col_span) = convert_columns_string(l_col, call.head)?;
    let (r_col_string, r_col_span) = convert_columns_string(r_col, call.head)?;

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;
    let r_df = NuDataFrame::try_from_value(r_df)?;

    check_column_datatypes(
        df.as_ref(),
        r_df.as_ref(),
        &l_col_string,
        l_col_span,
        &r_col_string,
        r_col_span,
    )?;

    df.as_ref()
        .join(
            r_df.as_ref(),
            &l_col_string,
            &r_col_string,
            join_type,
            suffix,
        )
        .map_err(|e| {
            ShellError::SpannedLabeledError(
                "Error joining dataframes".into(),
                e.to_string(),
                l_col_span,
            )
        })
        .map(|df| PipelineData::Value(NuDataFrame::dataframe_into_value(df, call.head), None))
}

fn check_column_datatypes<T: AsRef<str>>(
    df_l: &polars::prelude::DataFrame,
    df_r: &polars::prelude::DataFrame,
    l_cols: &[T],
    l_col_span: Span,
    r_cols: &[T],
    r_col_span: Span,
) -> Result<(), ShellError> {
    if l_cols.len() != r_cols.len() {
        return Err(ShellError::SpannedLabeledErrorHelp(
            "Mismatched number of column names".into(),
            format!(
                "found {} left names vs {} right names",
                l_cols.len(),
                r_cols.len()
            ),
            l_col_span,
            "perhaps you need to change the number of columns to join".into(),
        ));
    }

    for (l, r) in l_cols.iter().zip(r_cols) {
        let l_series = df_l.column(l.as_ref()).map_err(|e| {
            ShellError::SpannedLabeledError(
                "Error selecting the columns".into(),
                e.to_string(),
                l_col_span,
            )
        })?;

        let r_series = df_r.column(r.as_ref()).map_err(|e| {
            ShellError::SpannedLabeledError(
                "Error selecting the columns".into(),
                e.to_string(),
                r_col_span,
            )
        })?;

        if l_series.dtype() != r_series.dtype() {
            return Err(ShellError::SpannedLabeledErrorHelp(
                "Mismatched datatypes".into(),
                format!(
                    "left column type '{}' doesn't match '{}' right column match",
                    l_series.dtype(),
                    r_series.dtype()
                ),
                l_col_span,
                "perhaps you need to select other column to match".into(),
            ));
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
        test_dataframe(vec![Box::new(JoinDF {})])
    }
}
