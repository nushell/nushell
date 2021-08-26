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
        "dataframe drop"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Creates a new dataframe by dropping the selected columns"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe drop").rest(
            "rest",
            SyntaxShape::Any,
            "column names to be dropped",
        )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "drop column a",
            example: "[[a b]; [1 2] [3 4]] | dataframe to-df | dataframe drop a",
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![Column::new(
                    "b".to_string(),
                    vec![UntaggedValue::int(2).into(), UntaggedValue::int(4).into()],
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

    let columns: Vec<Value> = args.rest(0)?;
    let (col_string, col_span) = convert_columns(&columns, &tag)?;

    let (df, _) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let new_df = match col_string.get(0) {
        Some(col) => df
            .as_ref()
            .drop(col)
            .map_err(|e| parse_polars_error::<&str>(&e, &col_span, None)),
        None => Err(ShellError::labeled_error(
            "Empty names list",
            "No column names where found",
            &col_span,
        )),
    }?;

    // If there are more columns in the drop selection list, these
    // are added from the resulting dataframe
    let res = col_string.iter().skip(1).try_fold(new_df, |new_df, col| {
        new_df
            .drop(col)
            .map_err(|e| parse_polars_error::<&str>(&e, &col_span, None))
    })?;

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
