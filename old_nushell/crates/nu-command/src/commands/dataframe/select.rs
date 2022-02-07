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
        "dataframe select"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Creates a new dataframe with the selected columns"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe select").rest("rest", SyntaxShape::Any, "selected column names")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Create new dataframe with column a",
            example: "[[a b]; [1 2] [3 4]] | dataframe to-df | dataframe select a",
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

    let columns: Vec<Value> = args.rest(0)?;

    let (col_string, col_span) = convert_columns(&columns, &tag)?;

    let (df, _) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let res = df
        .as_ref()
        .select(&col_string)
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
