use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{NuDataFrame, PolarsData},
    Signature, TaggedDictBuilder, UntaggedValue,
};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "pls dtypes"
    }

    fn usage(&self) -> &str {
        "Show dataframe data types"
    }

    fn signature(&self) -> Signature {
        Signature::build("pls dtypes")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "drop column a",
            example: "echo [[a b]; [1 2] [3 4]] | pls convert | pls dtypes",
            result: None,
        }]
    }
}

fn command(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let mut args = args.evaluate_once()?;

    match args.input.next() {
        None => Err(ShellError::labeled_error(
            "No input received",
            "missing dataframe input from stream",
            &tag,
        )),
        Some(value) => {
            if let UntaggedValue::DataFrame(PolarsData::EagerDataFrame(NuDataFrame {
                dataframe: Some(df),
                ..
            })) = value.value
            {
                let col_names = df
                    .get_column_names()
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>();

                let values =
                    df.dtypes()
                        .into_iter()
                        .zip(col_names.into_iter())
                        .map(move |(dtype, name)| {
                            let mut data = TaggedDictBuilder::new(tag.clone());
                            data.insert_value("column", name.as_ref());
                            data.insert_value("dtype", format!("{}", dtype));

                            data.into_value()
                        });

                Ok(OutputStream::from_stream(values))
            } else {
                Err(ShellError::labeled_error(
                    "No dataframe in stream",
                    "no dataframe found in input stream",
                    &tag,
                ))
            }
        }
    }
}
