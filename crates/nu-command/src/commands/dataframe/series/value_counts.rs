use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{Column, NuDataFrame},
    Signature, UntaggedValue,
};

use crate::commands::dataframe::utils::parse_polars_error;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe value-counts"
    }

    fn usage(&self) -> &str {
        "[Series] Returns a dataframe with the counts for unique values in series"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe value-counts")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Calculates value counts",
            example: "[5 5 5 5 6 6] | dataframe to-df | dataframe value-counts",
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![
                    Column::new(
                        "0".to_string(),
                        vec![UntaggedValue::int(5).into(), UntaggedValue::int(6).into()],
                    ),
                    Column::new(
                        "counts".to_string(),
                        vec![UntaggedValue::int(4).into(), UntaggedValue::int(2).into()],
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

    let (df, df_tag) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let df_new = df
        .as_series(&df_tag.span)?
        .value_counts()
        .map_err(|e| parse_polars_error::<&str>(&e, &tag.span, None))?;

    Ok(OutputStream::one(NuDataFrame::dataframe_to_value(
        df_new, tag,
    )))
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
