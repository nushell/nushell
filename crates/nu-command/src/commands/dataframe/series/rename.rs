use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuSeries, Signature, SyntaxShape};
use nu_source::Tagged;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe rename"
    }

    fn usage(&self) -> &str {
        "[Series] Renames a series"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe rename").required(
            "name",
            SyntaxShape::String,
            "new series name",
        )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Renames a series",
            example: "[5 6 7 8] | dataframe to-series | dataframe rename-series new_name",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let name: Tagged<String> = args.req(0)?;

    let mut series = NuSeries::try_from_stream(&mut args.input, &tag.span)?;

    series.as_mut().rename(name.item.as_ref());

    Ok(OutputStream::one(series.into_value(tag)))
}
