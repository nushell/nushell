use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{NuDataFrame, PolarsStruct},
    Signature, UntaggedValue,
};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe groupby"
    }

    fn usage(&self) -> &str {
        "Creates a groupby operation on a dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe groupby")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        groupby(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Takes an input stream and converts it to a dataframe",
            example: "echo [[a b];[1 2] [3 4]] | dataframe",
            result: None,
        }]
    }
}

fn groupby(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {}
