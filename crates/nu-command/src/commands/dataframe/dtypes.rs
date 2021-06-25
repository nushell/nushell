use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature, TaggedDictBuilder};

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
            result: None,
        }]
    }
}

#[allow(clippy::needless_collect)]
fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let df = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;
    let col_names = df
        .as_ref()
        .get_column_names()
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<String>>();

    let values = df
        .as_ref()
        .dtypes()
        .into_iter()
        .zip(col_names.into_iter())
        .map(move |(dtype, name)| {
            let mut data = TaggedDictBuilder::new(tag.clone());
            data.insert_value("column", name.as_ref());
            data.insert_value("dtype", format!("{}", dtype));

            data.into_value()
        });

    Ok(OutputStream::from_stream(values))
}
