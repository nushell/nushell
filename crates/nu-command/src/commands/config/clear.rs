use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue};

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
    let name = args.call_info.name_tag.clone();
    let ctx = &args.context;

    let result = if let Some(global_cfg) = &mut args.configs().lock().global_config {
        global_cfg.vars.clear();
        global_cfg.write()?;
        ctx.reload_config(global_cfg)?;

        let value = UntaggedValue::Row(global_cfg.vars.clone().into()).into_value(name);
        Ok(OutputStream::one(value))
    } else {
        let value = UntaggedValue::Error(crate::commands::config::err_no_global_cfg_present())
            .into_value(name);

        Ok(OutputStream::one(value))
    };

    result
}
