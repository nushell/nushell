use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{Column, NuDataFrame},
    Signature, UntaggedValue,
};

use super::utils::parse_polars_error;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe to-dummies"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Creates a new dataframe with dummy variables"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe to-dummies")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create new dataframe with dummy variables from a dataframe",
                example: "[[a b]; [1 2] [3 4]] | dataframe to-df | dataframe to-dummies",
                result: Some(vec![NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "a_1".to_string(),
                            vec![UntaggedValue::int(1).into(), UntaggedValue::int(0).into()],
                        ),
                        Column::new(
                            "a_3".to_string(),
                            vec![UntaggedValue::int(0).into(), UntaggedValue::int(1).into()],
                        ),
                        Column::new(
                            "b_2".to_string(),
                            vec![UntaggedValue::int(1).into(), UntaggedValue::int(0).into()],
                        ),
                        Column::new(
                            "b_4".to_string(),
                            vec![UntaggedValue::int(0).into(), UntaggedValue::int(1).into()],
                        ),
                    ],
                    &Span::default(),
                )
                .expect("simple df for test should not fail")
                .into_value(Tag::default())]),
            },
            Example {
                description: "Create new dataframe with dummy variables from a series",
                example: "[1 2 2 3 3] | dataframe to-df | dataframe to-dummies",
                result: Some(vec![NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "0_1".to_string(),
                            vec![
                                UntaggedValue::int(1).into(),
                                UntaggedValue::int(0).into(),
                                UntaggedValue::int(0).into(),
                                UntaggedValue::int(0).into(),
                                UntaggedValue::int(0).into(),
                            ],
                        ),
                        Column::new(
                            "0_2".to_string(),
                            vec![
                                UntaggedValue::int(0).into(),
                                UntaggedValue::int(1).into(),
                                UntaggedValue::int(1).into(),
                                UntaggedValue::int(0).into(),
                                UntaggedValue::int(0).into(),
                            ],
                        ),
                        Column::new(
                            "0_3".to_string(),
                            vec![
                                UntaggedValue::int(0).into(),
                                UntaggedValue::int(0).into(),
                                UntaggedValue::int(0).into(),
                                UntaggedValue::int(1).into(),
                                UntaggedValue::int(1).into(),
                            ],
                        ),
                    ],
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

    match value.value {
        UntaggedValue::DataFrame(df) => {
            let res = df.as_ref().to_dummies().map_err(|e| {
                parse_polars_error(
                    &e,
                    &tag.span,
                    Some("The only allowed column types for dummies are String or Int"),
                )
            })?;

            Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
        }
        _ => Err(ShellError::labeled_error(
            "Incorrect type",
            "dummies cannot be done with this value",
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
