use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue};

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "config path"
    }

    fn signature(&self) -> Signature {
        Signature::build("config path")
    }

    fn usage(&self) -> &str {
        "return the path to the config file"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        path(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get the path to the current config file",
            example: "config path",
            result: None,
        }]
    }
}

pub fn path(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if let Some(global_cfg) = &mut args.configs.lock().global_config {
        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::Primitive(Primitive::FilePath(global_cfg.file_path.clone())),
        )))
    } else {
        Ok(vec![ReturnSuccess::value(UntaggedValue::Error(
            crate::commands::config::err_no_global_cfg_present(),
        ))]
        .into_iter()
        .to_output_stream())
    }
}
