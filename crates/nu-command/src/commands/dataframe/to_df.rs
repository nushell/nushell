use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{Column, NuDataFrame},
    Signature, UntaggedValue,
};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe to-df"
    }

    fn usage(&self) -> &str {
        "Converts a List, Table or Dictionary into a polars dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe to-df")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();

        let df = NuDataFrame::try_from_iter(args.input, &tag)?;

        Ok(InputStream::one(df.into_value(tag)))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Takes a dictionary and creates a dataframe",
                example: "[[a b];[1 2] [3 4]] | dataframe to-df",
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
                description: "Takes a list of tables and creates a dataframe",
                example: "[[1 2 a] [3 4 b] [5 6 c]] | dataframe to-df",
                result: Some(vec![NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "0".to_string(),
                            vec![
                                UntaggedValue::int(1).into(),
                                UntaggedValue::int(3).into(),
                                UntaggedValue::int(5).into(),
                            ],
                        ),
                        Column::new(
                            "1".to_string(),
                            vec![
                                UntaggedValue::int(2).into(),
                                UntaggedValue::int(4).into(),
                                UntaggedValue::int(6).into(),
                            ],
                        ),
                        Column::new(
                            "2".to_string(),
                            vec![
                                UntaggedValue::string("a").into(),
                                UntaggedValue::string("b").into(),
                                UntaggedValue::string("c").into(),
                            ],
                        ),
                    ],
                    &Span::default(),
                )
                .expect("simple df for test should not fail")
                .into_value(Tag::default())]),
            },
            Example {
                description: "Takes a list and creates a dataframe",
                example: "[a b c] | dataframe to-df",
                result: Some(vec![NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "0".to_string(),
                        vec![
                            UntaggedValue::string("a").into(),
                            UntaggedValue::string("b").into(),
                            UntaggedValue::string("c").into(),
                        ],
                    )],
                    &Span::default(),
                )
                .expect("simple df for test should not fail")
                .into_value(Tag::default())]),
            },
            Example {
                description: "Takes a list of booleans and creates a dataframe",
                example: "[$true $true $false] | dataframe to-df",
                result: Some(vec![NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "0".to_string(),
                        vec![
                            UntaggedValue::boolean(true).into(),
                            UntaggedValue::boolean(true).into(),
                            UntaggedValue::boolean(false).into(),
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
