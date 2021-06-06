use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature, SyntaxShape};

use nu_source::Tagged;
pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "pls slice"
    }

    fn usage(&self) -> &str {
        "Creates new dataframe from a slice of rows"
    }

    fn signature(&self) -> Signature {
        Signature::build("pls select")
            .required("offset", SyntaxShape::Number, "start of slice")
            .required("size", SyntaxShape::Number, "size of slice")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Create new dataframe from a slice of the rows",
            example: "[[a b]; [1 2] [3 4]] | pls to_df | pls slice 0 1",
            result: None,
        }]
    }
}

fn command(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let mut args = args.evaluate_once()?;

    let offset: Tagged<usize> = args.req(0)?;
    let size: Tagged<usize> = args.req(1)?;

    let df = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;
    let res = df.as_ref().slice(offset.item as i64, size.item);

    Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
}
