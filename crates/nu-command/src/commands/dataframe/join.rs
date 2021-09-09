use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{Column, NuDataFrame},
    Signature, SyntaxShape, UntaggedValue, Value,
};

use super::utils::{convert_columns, parse_polars_error};

use polars::prelude::JoinType;

use nu_source::Tagged;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe join"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Joins a dataframe using columns as reference"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe join")
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
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "inner join dataframe",
            example: r#"let right = ([[a b c]; [1 2 5] [3 4 5] [5 6 6]] | dataframe to-df);
    $right | dataframe join $right -l [a b] -r [a b]"#,
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![
                    Column::new(
                        "a".to_string(),
                        vec![
                            UntaggedValue::int(1).into(),
                            UntaggedValue::int(3).into(),
                            UntaggedValue::int(5).into(),
                        ],
                    ),
                    Column::new(
                        "b".to_string(),
                        vec![
                            UntaggedValue::int(2).into(),
                            UntaggedValue::int(4).into(),
                            UntaggedValue::int(6).into(),
                        ],
                    ),
                    Column::new(
                        "c".to_string(),
                        vec![
                            UntaggedValue::int(5).into(),
                            UntaggedValue::int(5).into(),
                            UntaggedValue::int(6).into(),
                        ],
                    ),
                    Column::new(
                        "c_right".to_string(),
                        vec![
                            UntaggedValue::int(5).into(),
                            UntaggedValue::int(5).into(),
                            UntaggedValue::int(6).into(),
                        ],
                    ),
                ],
                &Span::default(),
            )
            .expect("simple df for test should not fail")
            .into_value(Tag::default())]),
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let r_df: Value = args.req(0)?;
    let l_col: Vec<Value> = args.req_named("left")?;
    let r_col: Vec<Value> = args.req_named("right")?;
    let join_type_op: Option<Tagged<String>> = args.get_flag("type")?;

    let join_type = match join_type_op {
        None => JoinType::Inner,
        Some(val) => match val.item.as_ref() {
            "inner" => JoinType::Inner,
            "outer" => JoinType::Outer,
            "left" => JoinType::Left,
            _ => {
                return Err(ShellError::labeled_error_with_secondary(
                    "Incorrect join type",
                    "Invalid join type",
                    &val.tag,
                    "Perhaps you mean: inner, outer or left",
                    &val.tag,
                ))
            }
        },
    };

    let (l_col_string, l_col_span) = convert_columns(&l_col, &tag)?;
    let (r_col_string, r_col_span) = convert_columns(&r_col, &tag)?;

    let (df, _) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let res = match r_df.value {
        UntaggedValue::DataFrame(r_df) => {
            // Checking the column types before performing the join
            check_column_datatypes(
                df.as_ref(),
                r_df.as_ref(),
                &l_col_string,
                &l_col_span,
                &r_col_string,
                &r_col_span,
            )?;

            df.as_ref()
                .join(r_df.as_ref(), &l_col_string, &r_col_string, join_type)
                .map_err(|e| parse_polars_error::<&str>(&e, &l_col_span, None))
        }
        _ => Err(ShellError::labeled_error(
            "Not a dataframe",
            "not a dataframe type value",
            &r_df.tag,
        )),
    }?;

    Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
}

fn check_column_datatypes<T: AsRef<str>>(
    df_l: &polars::prelude::DataFrame,
    df_r: &polars::prelude::DataFrame,
    l_cols: &[T],
    l_col_span: &Span,
    r_cols: &[T],
    r_col_span: &Span,
) -> Result<(), ShellError> {
    if l_cols.len() != r_cols.len() {
        return Err(ShellError::labeled_error_with_secondary(
            "Mismatched number of column names",
            format!(
                "found {} left names vs {} right names",
                l_cols.len(),
                r_cols.len()
            ),
            l_col_span,
            "perhaps you need to change the number of columns to join",
            r_col_span,
        ));
    }

    for (l, r) in l_cols.iter().zip(r_cols) {
        let l_series = df_l
            .column(l.as_ref())
            .map_err(|e| parse_polars_error::<&str>(&e, l_col_span, None))?;

        let r_series = df_r
            .column(r.as_ref())
            .map_err(|e| parse_polars_error::<&str>(&e, r_col_span, None))?;

        if l_series.dtype() != r_series.dtype() {
            return Err(ShellError::labeled_error_with_secondary(
                "Mismatched datatypes",
                format!(
                    "left column type '{}' doesn't match '{}' right column match",
                    l_series.dtype(),
                    r_series.dtype()
                ),
                l_col_span,
                "perhaps you need to select other column to match",
                r_col_span,
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::DataFrame;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test_dataframe as test_examples;

        test_examples(DataFrame {})
    }
}
