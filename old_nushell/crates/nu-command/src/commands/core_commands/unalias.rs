use crate::prelude::*;
use nu_engine::WholeStreamCommand;

use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};

pub struct Unalias;

impl WholeStreamCommand for Unalias {
    fn name(&self) -> &str {
        "unalias"
    }

    fn signature(&self) -> Signature {
        Signature::build("unalias").required("name", SyntaxShape::String, "the name of the alias")
    }

    fn usage(&self) -> &str {
        "Removes an alias"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        unalias(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Remove the 'v' alias",
            example: "unalias v",
            result: None,
        }]
    }
}

pub fn unalias(_: CommandArgs) -> Result<OutputStream, ShellError> {
    Ok(OutputStream::empty())
}
