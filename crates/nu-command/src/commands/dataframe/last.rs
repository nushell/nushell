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
        "dataframe last"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Creates new dataframe with tail rows"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe last").optional(
            "n_rows",
            SyntaxShape::Number,
            "Number of rows for tail",
        )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Create new dataframe with last rows",
            example: "[[a b]; [1 2] [3 4]] | dataframe to-df | dataframe last 1",
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![
                    Column::new("a".to_string(), vec![UntaggedValue::int(3).into()]),
                    Column::new("b".to_string(), vec![UntaggedValue::int(4).into()]),
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
    let rows: Option<Tagged<usize>> = args.opt(0)?;

    let rows = match rows {
        Some(val) => val.item,
        None => 5,
    };

    let (df, _) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let res = df.as_ref().tail(Some(rows));

    Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
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
