use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "config clear"
    }

    fn signature(&self) -> Signature {
        Signature::build("config clear")
    }

    fn usage(&self) -> &str {
        "clear the config"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        clear(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Clear the config (be careful!)",
            example: "config clear",
            result: None,
        }]
    }
}

pub fn clear(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctx = EvaluationContext::from_args(&args);

    let result = if let Some(global_cfg) = &mut args.configs.lock().global_config {
        global_cfg.vars.clear();
        global_cfg.write()?;
        ctx.reload_config(global_cfg)?;
        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::Row(global_cfg.vars.clone().into()).into_value(args.call_info.name_tag),
        )))
    } else {
        Ok(vec![ReturnSuccess::value(UntaggedValue::Error(
            crate::commands::config::err_no_global_cfg_present(),
        ))]
        .into_iter()
        .to_output_stream())
    };

    result
}
