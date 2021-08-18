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
        "dataframe get-month"
    }

    fn usage(&self) -> &str {
        "[Series] Gets month from date"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe get-month")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Returns month from a date",
            example: r#"let dt = ('2020-08-04T16:39:18+00:00' | str to-datetime -z 'UTC');
    let df = ([$dt $dt] | dataframe to-df);
    $df | dataframe get-month"#,
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![Column::new(
                    "0".to_string(),
                    vec![UntaggedValue::int(8).into(), UntaggedValue::int(8).into()],
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
    let series = df.as_series(&df_tag.span)?;

    let casted = series
        .date64()
        .map_err(|e| parse_polars_error::<&str>(&e, &df_tag.span, None))?;

    let res = casted.month().into_series();
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
