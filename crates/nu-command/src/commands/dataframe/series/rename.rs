use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{Column, NuDataFrame},
    Signature, SyntaxShape, UntaggedValue,
};
use nu_source::Tagged;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe rename"
    }

    fn usage(&self) -> &str {
        "[Series] Renames a series"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe rename").required(
            "name",
            SyntaxShape::String,
            "new series name",
        )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Renames a series",
            example: "[5 6 7 8] | dataframe to-df | dataframe rename new_name",
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![Column::new(
                    "new_name".to_string(),
                    vec![
                        UntaggedValue::int(5).into(),
                        UntaggedValue::int(6).into(),
                        UntaggedValue::int(7).into(),
                        UntaggedValue::int(8).into(),
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
    let name: Tagged<String> = args.req(0)?;

    let (df, df_tag) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let mut series = df.as_series(&df_tag.span)?;

    series.rename(&name.item);

    let df = NuDataFrame::try_from_series(vec![series], &tag.span)?;
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
