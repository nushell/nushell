use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuSeries, Signature, SyntaxShape};
use nu_source::Tagged;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe to-series"
    }

    fn usage(&self) -> &str {
        "Converts a pipelined List into a polars series"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe to-series").optional(
            "name",
            SyntaxShape::String,
            "Optional series name",
        )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();

        let name: Option<Tagged<String>> = args.opt(0)?;
        let name = name.map(|v| v.item);

        let series = NuSeries::try_from_iter(args.input, name)?;

        Ok(InputStream::one(series.into_value(tag)))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Takes an input stream and converts it to a polars series",
            example: "[1 2 3 4] | dataframe to-series my-col",
            result: None,
        }]
    }
}
