use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{Column, NuDataFrame},
    Signature, SyntaxShape, UntaggedValue, Value,
};
use polars::prelude::IntoSeries;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe concatenate"
    }

    fn usage(&self) -> &str {
        "[Series] Concatenates strings with other array"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe concatenate").required(
            "other",
            SyntaxShape::Any,
            "Other array with string to be concatenated",
        )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Concatenate string",
            example: r#"let other = ([za xs cd] | dataframe to-df);
    [abc abc abc] | dataframe to-df | dataframe concatenate $other"#,
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![Column::new(
                    "0".to_string(),
                    vec![
                        UntaggedValue::string("abcza").into(),
                        UntaggedValue::string("abcxs").into(),
                        UntaggedValue::string("abccd").into(),
                    ],
                )],
                &Span::default(),
            )
            .expect("simple df for test should not fail")
            .into_value(Tag::default())]),
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let other: Value = args.req(0)?;

    let other_df = match &other.value {
        UntaggedValue::DataFrame(df) => Ok(df),
        _ => Err(ShellError::labeled_error(
            "Incorrect type",
            "can only concatenate another series",
            other.tag.span,
        )),
    }?;

    let other_series = other_df.as_series(&other.tag.span)?;
    let other_chunked = other_series.utf8().map_err(|e| {
        parse_polars_error::<&str>(
            &e,
            &other.tag.span,
            Some("The concatenate command can only be used with string columns"),
        )
    })?;

    let (df, df_tag) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let series = df.as_series(&df_tag.span)?;
    let chunked = series.utf8().map_err(|e| {
        parse_polars_error::<&str>(
            &e,
            &df_tag.span,
            Some("The concatenate command can only be used with string columns"),
        )
    })?;

    let mut res = chunked.concat(other_chunked);

    res.rename(series.name());

    let df = NuDataFrame::try_from_series(vec![res.into_series()], &tag.span)?;
    Ok(OutputStream::one(df.into_value(df_tag)))
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
