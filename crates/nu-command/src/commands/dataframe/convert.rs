use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{NuDataFrame, PolarsData},
    Signature, UntaggedValue,
};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "pls convert"
    }

    fn usage(&self) -> &str {
        "Converts a pipelined Table or List into a polars dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build("pls convert")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let args = args.evaluate_once()?;

        let df = NuDataFrame::try_from_iter(args.input, &tag)?;
        let init = InputStream::one(
            UntaggedValue::DataFrame(PolarsData::EagerDataFrame(df)).into_value(&tag),
        );

        Ok(init.to_output_stream())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Takes an input stream and converts it to a polars dataframe",
            example: "echo [[a b];[1 2] [3 4]] | pls convert",
            result: None,
        }]
    }
}
