use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{Column, NuDataFrame},
    Signature, SyntaxShape, UntaggedValue, Value,
};

use super::utils::{convert_columns, parse_polars_error};
pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe sort"
    }

    fn usage(&self) -> &str {
        "[DataFrame, Series] Creates new sorted dataframe or series"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe sort")
            .switch("reverse", "invert sort", Some('r'))
            .rest("rest", SyntaxShape::Any, "column names to sort dataframe")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create new sorted dataframe",
                example: "[[a b]; [3 4] [1 2]] | dataframe to-df | dataframe sort a",
                result: Some(vec![NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "a".to_string(),
                            vec![UntaggedValue::int(1).into(), UntaggedValue::int(3).into()],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![UntaggedValue::int(2).into(), UntaggedValue::int(4).into()],
                        ),
                    ],
                    &Span::default(),
                )
                .expect("simple df for test should not fail")
                .into_value(Tag::default())]),
            },
            Example {
                description: "Create new sorted series",
                example: "[3 4 1 2] | dataframe to-df | dataframe sort",
                result: Some(vec![NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "0".to_string(),
                        vec![
                            UntaggedValue::int(1).into(),
                            UntaggedValue::int(2).into(),
                            UntaggedValue::int(3).into(),
                            UntaggedValue::int(4).into(),
                        ],
                    )],
                    &Span::default(),
                )
                .expect("simple df for test should not fail")
                .into_value(Tag::default())]),
            },
        ]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let value = args.input.next().ok_or_else(|| {
        ShellError::labeled_error("Empty stream", "No value found in stream", &tag.span)
    })?;

    let reverse = args.has_flag("reverse");

    match &value.value {
        UntaggedValue::DataFrame(df) => {
            if df.is_series() {
                let columns = df.as_ref().get_column_names();

                let res = df
                    .as_ref()
                    .sort(columns, reverse)
                    .map_err(|e| parse_polars_error::<&str>(&e, &value.tag.span, None))?;

                Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
            } else {
                let columns: Vec<Value> = args.rest(0)?;

                if !columns.is_empty() {
                    let (col_string, col_span) = convert_columns(&columns, &tag)?;

                    let res = df
                        .as_ref()
                        .sort(&col_string, reverse)
                        .map_err(|e| parse_polars_error::<&str>(&e, &col_span, None))?;

                    Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
                } else {
                    Err(ShellError::labeled_error(
                        "Missing columns",
                        "missing column name to perform sort",
                        &tag.span,
                    ))
                }
            }
        }
        _ => Err(ShellError::labeled_error(
            "Incorrect type",
            "sort cannot be done with this value",
            &value.tag.span,
        )),
    }
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
