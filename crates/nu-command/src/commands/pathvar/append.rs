use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;
use nu_test_support::{NATIVE_PATH_ENV_SEPARATOR, NATIVE_PATH_ENV_VAR};
use std::path::PathBuf;

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "pathvar append"
    }

    fn signature(&self) -> Signature {
        Signature::build("pathvar append").required("path", SyntaxShape::FilePath, "path to append")
    }

    fn usage(&self) -> &str {
        "Add a path to the end of the pathvar"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        add(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Append /bin to the pathvar",
            example: "pathvar append /bin",
            result: None,
        }]
    }
}

pub fn add(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctx = &args.context;
    let path_to_append_arg: Tagged<PathBuf> = args.req(0)?;
    let path_to_append = path_to_append_arg.item.into_os_string().into_string();

    if let Ok(path) = path_to_append {
        if let Some(mut pathvar) = ctx.scope.get_env(NATIVE_PATH_ENV_VAR) {
            pathvar.push(NATIVE_PATH_ENV_SEPARATOR);
            pathvar.push_str(&path);
            ctx.scope.add_env_var(NATIVE_PATH_ENV_VAR, pathvar);
            Ok(OutputStream::empty())
        } else {
            Err(ShellError::unexpected("PATH not set"))
        }
    } else {
        Err(ShellError::labeled_error(
            "Invalid path.",
            "cannot convert to string",
            path_to_append_arg.tag,
        ))
    }
}
