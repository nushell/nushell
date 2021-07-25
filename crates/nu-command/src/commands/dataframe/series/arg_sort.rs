use crate::prelude::*;
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
        "dataframe arg-sort"
    }

    fn usage(&self) -> &str {
        "[Series] Returns indexes for a sorted series"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe arg-sort").switch("reverse", "reverse order", Some('r'))
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Returns indexes for a sorted series",
            example: "[1 2 2 3 3] | dataframe to-df | dataframe arg-sort",
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![Column::new(
                    "arg_sort".to_string(),
                    vec![
                        UntaggedValue::int(0).into(),
                        UntaggedValue::int(1).into(),
                        UntaggedValue::int(2).into(),
                        UntaggedValue::int(3).into(),
                        UntaggedValue::int(4).into(),
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
    let reverse = args.has_flag("reverse");

    let (df, df_tag) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let mut res = df.as_series(&df_tag.span)?.argsort(reverse).into_series();
    res.rename("arg_sort");

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
