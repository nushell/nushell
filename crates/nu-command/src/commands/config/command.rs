use crate::prelude::*;
use nu_engine::CommandArgs;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
use nu_stream::ActionStream;

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

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        let name = args.call_info.name_tag.clone();

        if let Some(global_cfg) = &args.configs().lock().global_config {
            let result = global_cfg.vars.clone();
            Ok(vec![ReturnSuccess::value(
                UntaggedValue::Row(result.into()).into_value(name),
            )]
            .into_iter()
            .to_action_stream())
        } else {
            Ok(vec![ReturnSuccess::value(UntaggedValue::Error(
                crate::commands::config::err_no_global_cfg_present(),
            ))]
            .into_iter()
            .to_action_stream())
        }
    }
}
