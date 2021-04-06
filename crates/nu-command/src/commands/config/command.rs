use crate::prelude::*;
use nu_engine::CommandArgs;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
use nu_stream::OutputStream;

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
        let name = args.call_info.name_tag;

        if let Some(global_cfg) = &args.configs.lock().global_config {
            let result = global_cfg.vars.clone();
            Ok(vec![ReturnSuccess::value(
                UntaggedValue::Row(result.into()).into_value(name),
            )]
            .into_iter()
            .to_output_stream())
        } else {
            Ok(vec![ReturnSuccess::value(UntaggedValue::Error(
                ShellError::untagged_runtime_error("No global config found!"),
            ))]
            .into_iter()
            .to_output_stream())
        }
    }
}
