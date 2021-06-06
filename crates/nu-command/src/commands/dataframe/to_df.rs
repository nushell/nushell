use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "pls to_df"
    }

    fn usage(&self) -> &str {
        "Converts a pipelined Table or List into a polars dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build("pls to_df")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let args = args.evaluate_once()?;

        let df = NuDataFrame::try_from_iter(args.input, &tag)?;

        Ok(InputStream::one(df.to_value(tag)))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Takes an input stream and converts it to a polars dataframe",
            example: "[[a b];[1 2] [3 4]] | pls to_df",
            result: None,
        }]
    }
}
