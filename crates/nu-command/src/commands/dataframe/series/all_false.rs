use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{Column, NuDataFrame},
    Signature, UntaggedValue, Value,
};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe all-false"
    }

    fn usage(&self) -> &str {
        "[Series] Returns true if all values are false"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe all-false")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Returns true if all values are false",
                example: "[$false $false $false] | dataframe to-df | dataframe all-false",
                result: Some(vec![NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "all_false".to_string(),
                        vec![UntaggedValue::boolean(true).into()],
                    )],
                    &Span::default(),
                )
                .expect("simple df for test should not fail")
                .into_value(Tag::default())]),
            },
            Example {
                description: "Checks the result from a comparison",
                example: r#"let s = ([5 6 2 10] | dataframe to-df);
    let res = ($s > 9);
    $res | dataframe all-false"#,
                result: Some(vec![NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "all_false".to_string(),
                        vec![UntaggedValue::boolean(false).into()],
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

    let (df, df_tag) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let series = df.as_series(&df_tag.span)?;
    let bool = series.bool().map_err(|e| {
        parse_polars_error::<&str>(
            &e,
            &tag.span,
            Some("all-false only works with series of type bool"),
        )
    })?;

    let res = bool.all_false();

    let value = Value {
        value: UntaggedValue::Primitive(res.into()),
        tag: tag.clone(),
    };

    let df = NuDataFrame::try_from_columns(
        vec![Column::new("all_false".to_string(), vec![value])],
        &tag.span,
    )?;

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
