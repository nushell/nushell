use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue};
use nu_test_support::{NATIVE_PATH_ENV_SEPARATOR, NATIVE_PATH_ENV_VAR};

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "pathvar reset"
    }

    fn signature(&self) -> Signature {
        Signature::build("pathvar reset")
    }

    fn usage(&self) -> &str {
        "Reset the pathvar to the one specified in the config"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        reset(args)
    }
}
pub fn reset(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let ctx = &args.context;

    if let Some(global_cfg) = &mut ctx.configs().lock().global_config {
        let default_pathvar = global_cfg.vars.get("path");
        if let Some(pathvar) = default_pathvar {
            if let UntaggedValue::Table(paths) = &pathvar.value {
                let pathvar_str = paths
                    .iter()
                    .map(|x| x.as_string().expect("Error converting path to string"))
                    .join(&NATIVE_PATH_ENV_SEPARATOR.to_string());
                ctx.scope.add_env_var(NATIVE_PATH_ENV_VAR, pathvar_str);
            }
        } else {
            return Err(ShellError::untagged_runtime_error(
                "Default path is not set in config file.",
            ));
        }
        Ok(OutputStream::empty())
    } else {
        let value = UntaggedValue::Error(crate::commands::config::err_no_global_cfg_present())
            .into_value(name);

        Ok(OutputStream::one(value))
    }
}
