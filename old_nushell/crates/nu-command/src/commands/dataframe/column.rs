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
        "dataframe column"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Returns the selected column as Series"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe column").required("column", SyntaxShape::String, "column name")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Returns the selected column as series",
            example: "[[a b]; [1 2] [3 4]] | dataframe to-df | dataframe column a",
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![Column::new(
                    "a".to_string(),
                    vec![UntaggedValue::int(1).into(), UntaggedValue::int(3).into()],
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
    let column: Tagged<String> = args.req(0)?;

    let (df, df_tag) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let res = df
        .as_ref()
        .column(&column.item)
        .map_err(|e| parse_polars_error::<&str>(&e, &column.tag.span, None))?;

    let df = NuDataFrame::try_from_series(vec![res.clone()], &tag.span)?;
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
