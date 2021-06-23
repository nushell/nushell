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
        "pathvar add"
    }

    fn signature(&self) -> Signature {
        Signature::build("pathvar add").required("path", SyntaxShape::FilePath, "path to add")
    }

    fn usage(&self) -> &str {
        "Add a filepath to the start of the pathvar"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        add(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Add /usr/local/bin to the pathvar",
            example: "pathvar add /usr/local/bin",
            result: None,
        }]
    }
}

pub fn add(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctx = &args.context;

    let path_to_add: Tagged<PathBuf> = args.req(0)?;
    let path = path_to_add.item.into_os_string().into_string();

    if let Ok(mut path) = path {
        path.push(NATIVE_PATH_ENV_SEPARATOR);
        if let Some(old_pathvar) = ctx.scope.get_env(NATIVE_PATH_ENV_VAR) {
            path.push_str(&old_pathvar);
            ctx.scope.add_env_var(NATIVE_PATH_ENV_VAR, path);
            Ok(OutputStream::empty())
        } else {
            Err(ShellError::unexpected("PATH not set"))
        }
    } else {
        Err(ShellError::labeled_error(
            "Invalid path.",
            "cannot convert to string",
            path_to_add.tag,
        ))
    }
}
