use crate::prelude::*;
use nu_engine::CommandArgs;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue};

pub struct Command;

impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "config"
    }

    fn signature(&self) -> Signature {
        Signature::build("config")
    }

    fn usage(&self) -> &str {
        "Configuration management."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let name = args.call_info.name_tag.clone();

        if let Some(global_cfg) = &args.configs().lock().global_config {
            let result = global_cfg.vars.clone();
            let value = UntaggedValue::Row(result.into()).into_value(name);

            Ok(OutputStream::one(value))
        } else {
            let value = UntaggedValue::Error(crate::commands::config::err_no_global_cfg_present())
                .into_value(name);

            Ok(OutputStream::one(value))
        }
    }
}
