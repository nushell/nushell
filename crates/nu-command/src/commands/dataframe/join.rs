use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{NuDataFrame, PolarsData},
    Signature, SyntaxShape, UntaggedValue, Value,
};

use super::utils::{convert_columns, parse_polars_error};

use polars::prelude::JoinType;

use nu_source::Tagged;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "pls join"
    }

    fn usage(&self) -> &str {
        "Joins a dataframe using columns as reference"
    }

    fn signature(&self) -> Signature {
        Signature::build("pls join")
            .required("dataframe", SyntaxShape::Any, "right dataframe to join")
            .required(
                "l_columns",
                SyntaxShape::Table,
                "left column names to perform join",
            )
            .required(
                "r_columns",
                SyntaxShape::Table,
                "right column names to perform join",
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
        vec![
            Example {
                description: "inner join dataframe",
                example: "echo [[a b]; [1 2] [3 4]] | pls convert | pls join $right [a] [a]",
                result: None,
            },
            Example {
                description: "right join dataframe",
                example:
                    "echo [[a b]; [1 2] [3 4] [5 6]] | pls convert | pls join $right [b] [b] -t right",
                result: None,
            },
        ]
    }
}

fn command(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let mut args = args.evaluate_once()?;

    let r_df: Value = args.req(0)?;
    let l_col: Vec<Value> = args.req(1)?;
    let r_col: Vec<Value> = args.req(2)?;
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

    match args.input.next() {
        None => Err(ShellError::labeled_error(
            "No input received",
            "missing dataframe input from stream",
            &tag,
        )),
        Some(value) => {
            if let UntaggedValue::DataFrame(PolarsData::EagerDataFrame(df)) = value.value {
                let res = match r_df.value {
                    UntaggedValue::DataFrame(PolarsData::EagerDataFrame(r_df)) => {
                        // Checking the column types before performing the join
                        check_column_datatypes(
                            df.as_ref(),
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

                let value = Value {
                    value: UntaggedValue::DataFrame(PolarsData::EagerDataFrame(NuDataFrame::new(
                        res,
                    ))),
                    tag: tag.clone(),
                };

                Ok(OutputStream::one(value))
            } else {
                Err(ShellError::labeled_error(
                    "No dataframe in stream",
                    "no dataframe found in input stream",
                    &tag,
                ))
            }
        }
    }
}

fn check_column_datatypes<T: AsRef<str>>(
    df: &polars::prelude::DataFrame,
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

    for (l, r) in l_cols.iter().zip(r_cols.iter()) {
        let l_series = df
            .column(l.as_ref())
            .map_err(|e| parse_polars_error::<&str>(&e, &l_col_span, None))?;

        let r_series = df
            .column(r.as_ref())
            .map_err(|e| parse_polars_error::<&str>(&e, &r_col_span, None))?;

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
