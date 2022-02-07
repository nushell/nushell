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
        "dataframe is-in"
    }

    fn usage(&self) -> &str {
        "[Series] Checks if elements from a series are contained in right series"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe is-in").required("other", SyntaxShape::Any, "right series")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Checks if elements from a series are contained in right series",
            example: r#"let other = ([1 3 6] | dataframe to-df);
    [5 6 6 6 8 8 8] | dataframe to-df | dataframe is-in $other"#,
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![Column::new(
                    "is_in".to_string(),
                    vec![
                        UntaggedValue::boolean(false).into(),
                        UntaggedValue::boolean(true).into(),
                        UntaggedValue::boolean(true).into(),
                        UntaggedValue::boolean(true).into(),
                        UntaggedValue::boolean(false).into(),
                        UntaggedValue::boolean(false).into(),
                        UntaggedValue::boolean(false).into(),
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
    let value: Value = args.req(0)?;

    let other_df = match value.value {
        UntaggedValue::DataFrame(df) => Ok(df),
        _ => Err(ShellError::labeled_error(
            "Incorrect type",
            "can only search in a series",
            value.tag.span,
        )),
    }?;

    let other = other_df.as_series(&value.tag.span)?;

    let (df, df_tag) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let mut res = df
        .as_series(&df_tag.span)?
        .is_in(&other)
        .map_err(|e| parse_polars_error::<&str>(&e, &tag.span, None))?
        .into_series();

    res.rename("is_in");

    let df = NuDataFrame::try_from_series(vec![res], &tag.span)?;
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
