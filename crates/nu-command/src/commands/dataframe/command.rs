use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{NuDataFrame, PolarsStruct},
    Signature, UntaggedValue,
};

pub struct Command;

impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "dataframe"
    }

    fn usage(&self) -> &str {
        "Creates a dataframe from pipelined Table or List "
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let args = args.evaluate_once()?;

        let df = NuDataFrame::try_from_iter(args.input, &tag)?;
        let init =
            InputStream::one(UntaggedValue::Data(PolarsStruct::DataFrame(df)).into_value(&tag));

        Ok(init.to_output_stream())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Takes an input stream and converts it to a dataframe",
            example: "echo [[a b];[1 2] [3 4]] | dataframe",
            result: None,
        }]
    }
}
