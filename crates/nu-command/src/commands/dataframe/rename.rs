use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{Column, NuDataFrame},
    Signature, SyntaxShape, UntaggedValue,
};

use nu_source::Tagged;

use super::utils::parse_polars_error;
pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe rename-col"
    }

    fn usage(&self) -> &str {
        "[DataFrame] rename a dataframe column"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe rename-col")
            .required("from", SyntaxShape::String, "column name to be renamed")
            .required("to", SyntaxShape::String, "new column name")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Renames a dataframe column",
            example: "[[a b]; [1 2] [3 4]] | dataframe to-df | dataframe rename-col a ab",
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![
                    Column::new(
                        "ab".to_string(),
                        vec![UntaggedValue::int(1).into(), UntaggedValue::int(3).into()],
                    ),
                    Column::new(
                        "b".to_string(),
                        vec![UntaggedValue::int(2).into(), UntaggedValue::int(4).into()],
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
    let from: Tagged<String> = args.req(0)?;
    let to: Tagged<String> = args.req(1)?;

    let (mut df, df_tag) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    df.as_mut()
        .rename(&from.item, &to.item)
        .map_err(|e| parse_polars_error::<&str>(&e, &df_tag.span, None))?;

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
