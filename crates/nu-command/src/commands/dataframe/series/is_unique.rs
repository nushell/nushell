use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{Column, NuDataFrame},
    Signature, UntaggedValue,
};
use polars::prelude::IntoSeries;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe is-unique"
    }

    fn usage(&self) -> &str {
        "[Series] Creates mask indicating unique values"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe is-unique")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Create mask indicating unique values",
            example: "[5 6 6 6 8 8 8] | dataframe to-df | dataframe is-unique",
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![Column::new(
                    "is_unique".to_string(),
                    vec![
                        UntaggedValue::boolean(true).into(),
                        UntaggedValue::boolean(false).into(),
                        UntaggedValue::boolean(false).into(),
                        UntaggedValue::boolean(false).into(),
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

    let (df, df_tag) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let mut res = df
        .as_series(&df_tag.span)?
        .is_unique()
        .map_err(|e| parse_polars_error::<&str>(&e, &tag.span, None))?
        .into_series();

    res.rename("is_unique");

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
