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
        "dataframe dtypes"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Show dataframe data types"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe dtypes")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "drop column a",
            example: "[[a b]; [1 2] [3 4]] | dataframe to-df | dataframe dtypes",
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![
                    Column::new(
                        "column".to_string(),
                        vec![
                            UntaggedValue::string("a").into(),
                            UntaggedValue::string("b").into(),
                        ],
                    ),
                    Column::new(
                        "dtype".to_string(),
                        vec![
                            UntaggedValue::string("i64").into(),
                            UntaggedValue::string("i64").into(),
                        ],
                    ),
                ],
                &Span::default(),
            )
            .expect("simple df for test should not fail")
            .into_value(Tag::default())]),
        }]
    }
}

#[allow(clippy::needless_collect)]
fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let (df, _) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let mut dtypes: Vec<Value> = Vec::new();
    let names: Vec<Value> = df
        .as_ref()
        .get_column_names()
        .iter()
        .map(|v| {
            let dtype = df
                .as_ref()
                .column(v)
                .expect("using name from list of names from dataframe")
                .dtype();

            let dtype_str = dtype.to_string();
            dtypes.push(Value {
                value: dtype_str.into(),
                tag: Tag::default(),
            });

            Value {
                value: v.to_string().into(),
                tag: Tag::default(),
            }
        })
        .collect();

    let names_col = Column::new("column".to_string(), names);
    let dtypes_col = Column::new("dtype".to_string(), dtypes);

    let df = NuDataFrame::try_from_columns(vec![names_col, dtypes_col], &tag.span)?;
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
