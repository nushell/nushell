use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;

pub struct SubCommand;

#[derive(Deserialize)]
pub struct Arguments {
    remove: Tagged<String>,
}

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "config remove"
    }

    fn signature(&self) -> Signature {
        Signature::build("config remove").required(
            "remove",
            SyntaxShape::Any,
            "remove a value from the config",
        )
    }

    fn usage(&self) -> &str {
        "Removes a value from the config"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        remove(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Remove the startup commands",
            example: "config remove startup",
            result: None,
        }]
    }
}

pub fn remove(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let ctx = EvaluationContext::from_args(&args);
    let (Arguments { remove }, _) = args.process()?;

    let key = remove.to_string();

    let result = if let Some(global_cfg) = &mut ctx.configs.lock().global_config {
        if global_cfg.vars.contains_key(&key) {
            global_cfg.vars.swap_remove(&key);
            global_cfg.write()?;
            ctx.reload_config(global_cfg)?;
            Ok(vec![ReturnSuccess::value(
                UntaggedValue::row(global_cfg.vars.clone()).into_value(remove.tag()),
            )]
            .into_iter()
            .to_action_stream())
        } else {
            Err(ShellError::labeled_error(
                "Key does not exist in config",
                "key",
                remove.tag(),
            ))
        }
    } else {
        Ok(vec![ReturnSuccess::value(UntaggedValue::Error(
            crate::commands::config::err_no_global_cfg_present(),
        ))]
        .into_iter()
        .to_action_stream())
    };

    result
}
