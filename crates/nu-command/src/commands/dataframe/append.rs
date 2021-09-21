use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{Axis, Column, NuDataFrame},
    Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tagged;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe append"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Appends a new dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe append")
            .required_named(
                "other",
                SyntaxShape::Any,
                "dataframe to be appended",
                Some('o'),
            )
            .required_named(
                "axis",
                SyntaxShape::String,
                "row or col axis orientation",
                Some('a'),
            )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Appends a dataframe as new columns",
                example: r#"let a = ([[a b]; [1 2] [3 4]] | dataframe to-df);
    $a | dataframe append -o $a -a row"#,
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
                        Column::new(
                            "a_x".to_string(),
                            vec![UntaggedValue::int(1).into(), UntaggedValue::int(3).into()],
                        ),
                        Column::new(
                            "b_x".to_string(),
                            vec![UntaggedValue::int(2).into(), UntaggedValue::int(4).into()],
                        ),
                    ],
                    &Span::default(),
                )
                .expect("simple df for test should not fail")
                .into_value(Tag::default())]),
            },
            Example {
                description: "Appends a dataframe merging at the end of columns",
                example: r#"let a = ([[a b]; [1 2] [3 4]] | dataframe to-df);
    $a | dataframe append -o $a -a col"#,
                result: Some(vec![NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "a".to_string(),
                            vec![
                                UntaggedValue::int(1).into(),
                                UntaggedValue::int(3).into(),
                                UntaggedValue::int(1).into(),
                                UntaggedValue::int(3).into(),
                            ],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![
                                UntaggedValue::int(2).into(),
                                UntaggedValue::int(4).into(),
                                UntaggedValue::int(2).into(),
                                UntaggedValue::int(4).into(),
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
    let other: Value = args.req_named("other")?;
    let axis: Tagged<String> = args.req_named("axis")?;

    let axis = Axis::try_from_str(&axis.item, &axis.tag.span)?;

    let df_other = match other.value {
        UntaggedValue::DataFrame(df) => Ok(df),
        _ => Err(ShellError::labeled_error(
            "Incorrect type",
            "can only append a dataframe to a dataframe",
            other.tag.span,
        )),
    }?;

    let (df, _) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let df_new = df.append_df(&df_other, axis, &tag.span)?;
    Ok(OutputStream::one(df_new.into_value(tag)))
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
