use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{Column, NuDataFrame},
    Signature, UntaggedValue, Value,
};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe shape"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Shows column and row size for a dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe shape")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Shows row and column shape",
            example: "[[a b]; [1 2] [3 4]] | dataframe to-df | dataframe shape",
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![
                    Column::new("rows".to_string(), vec![UntaggedValue::int(2).into()]),
                    Column::new("columns".to_string(), vec![UntaggedValue::int(2).into()]),
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

    let (df, _) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let rows = Value {
        value: (df.as_ref().height() as i64).into(),
        tag: Tag::default(),
    };

    let cols = Value {
        value: (df.as_ref().width() as i64).into(),
        tag: Tag::default(),
    };

    let rows_col = Column::new("rows".to_string(), vec![rows]);
    let cols_col = Column::new("columns".to_string(), vec![cols]);

    let df = NuDataFrame::try_from_columns(vec![rows_col, cols_col], &tag.span)?;
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
