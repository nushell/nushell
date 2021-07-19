use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature, TaggedDictBuilder};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe size"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Shows column and row size for a dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe size")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Shows row and column size",
            example: "[[a b]; [1 2] [3 4]] | dataframe to-df | dataframe size",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let df = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let rows = df.as_ref().height();
    let cols = df.as_ref().width();

    let mut data = TaggedDictBuilder::new(&tag);
    data.insert_value("rows", format!("{}", rows));
    data.insert_value("columns", format!("{}", cols));

    Ok(OutputStream::one(data.into_value()))
}
