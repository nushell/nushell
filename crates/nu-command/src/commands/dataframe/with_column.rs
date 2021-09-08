use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{Column, NuDataFrame},
    Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tagged;

use super::utils::parse_polars_error;
pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe with-column"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Adds a series to the dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe with-column")
            .required("series", SyntaxShape::Any, "series to be added")
            .required_named("name", SyntaxShape::String, "column name", Some('n'))
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Adds a series to the dataframe",
            example:
                "[[a b]; [1 2] [3 4]] | dataframe to-df | dataframe with-column ([5 6] | dataframe to-df) --name c",
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![
                    Column::new(
                        "a".to_string(),
                        vec![
                            UntaggedValue::int(1).into(),
                            UntaggedValue::int(3).into(),
                        ],
                    ),
                    Column::new(
                        "b".to_string(),
                        vec![
                            UntaggedValue::int(2).into(),
                            UntaggedValue::int(4).into(),
                        ],
                    ),
                    Column::new(
                        "c".to_string(),
                        vec![
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
    let value: Value = args.req(0)?;
    let name: Tagged<String> = args.req_named("name")?;

    let df = match value.value {
        UntaggedValue::DataFrame(df) => Ok(df),
        _ => Err(ShellError::labeled_error(
            "Incorrect type",
            "can only add a series to a dataframe",
            value.tag.span,
        )),
    }?;

    let mut series = df.as_series(&value.tag.span)?;

    let series = series.rename(&name.item).clone();

    let (mut df, _) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    df.as_mut()
        .with_column(series)
        .map_err(|e| parse_polars_error::<&str>(&e, &tag.span, None))?;

    Ok(OutputStream::one(df.into_value(tag)))
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
