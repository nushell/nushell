use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{Column, NuDataFrame},
    Signature, SyntaxShape, UntaggedValue, Value,
};

use super::utils::{convert_columns, parse_polars_error};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe drop-duplicates"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Drops duplicate values in dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe drop-duplicates")
            .optional(
                "subset",
                SyntaxShape::Table,
                "subset of columns to drop duplicates",
            )
            .switch("maintain", "maintain order", Some('m'))
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "drop duplicates",
            example: "[[a b]; [1 2] [3 4] [1 2]] | dataframe to-df | dataframe drop-duplicates",
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![
                    Column::new(
                        "a".to_string(),
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

    // Extracting the selection columns of the columns to perform the aggregation
    let columns: Option<Vec<Value>> = args.opt(0)?;
    let (subset, col_span) = match columns {
        Some(cols) => {
            let (agg_string, col_span) = convert_columns(&cols, &tag)?;
            (Some(agg_string), col_span)
        }
        None => (None, Span::unknown()),
    };

    let (df, _) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let subset_slice = subset.as_ref().map(|cols| &cols[..]);

    let res = df
        .as_ref()
        .drop_duplicates(args.has_flag("maintain"), subset_slice)
        .map_err(|e| parse_polars_error::<&str>(&e, &col_span, None))?;

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
