use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe to-df"
    }

    fn usage(&self) -> &str {
        "Converts a List, Table or Dictionary into a polars dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe to-df")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();

        let df = NuDataFrame::try_from_iter(args.input, &tag)?;

        Ok(InputStream::one(df.into_value(tag)))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Takes a dictionary and creates a dataframe",
                example: "[[a b];[1 2] [3 4]] | dataframe to-df",
                result: None,
            },
            Example {
                description: "Takes a list of tables and creates a dataframe",
                example: "[[1 2 a] [3 4 b] [5 6 c]] | dataframe to-df",
                result: None,
            },
            Example {
                description: "Takes a list and creates a dataframe",
                example: "[a b c] | dataframe to-df",
                result: None,
            },
            Example {
                description: "Takes a list of booleans and creates a dataframe",
                example: "[true true false] | dataframe to-df",
                result: None,
            },
        ]
    }
}
