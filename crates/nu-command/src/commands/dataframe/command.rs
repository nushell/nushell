use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue};

pub struct Command;

impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "dataframe"
    }

    fn usage(&self) -> &str {
        "Commands to work with polars dataframes"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::one(
            UntaggedValue::string(get_full_help(&Command, args.scope())).into_value(Tag::unknown()),
        ))
    }
}
