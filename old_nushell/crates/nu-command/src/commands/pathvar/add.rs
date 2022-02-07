use super::get_var;
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;
use nu_test_support::NATIVE_PATH_ENV_SEPARATOR;
use std::path::PathBuf;

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "pathvar add"
    }

    fn signature(&self) -> Signature {
        Signature::build("pathvar add")
            .required("path", SyntaxShape::FilePath, "path to add")
            .named(
                "var",
                SyntaxShape::String,
                "Use a different variable than PATH",
                Some('v'),
            )
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

    let var = get_var(&args)?;
    let path_to_add: Tagged<PathBuf> = args.req(0)?;
    let path = path_to_add.item.into_os_string().into_string();

    if let Ok(mut path) = path {
        path.push(NATIVE_PATH_ENV_SEPARATOR);
        if let Some(old_pathvar) = ctx.scope.get_env(&var) {
            path.push_str(&old_pathvar);
            ctx.scope.add_env_var(&var.item, path);
            Ok(OutputStream::empty())
        } else {
            Err(ShellError::unexpected(&format!(
                "Variable {} not set",
                &var.item
            )))
        }
    } else {
        Err(ShellError::labeled_error(
            "Invalid path.",
            "cannot convert to string",
            path_to_add.tag,
        ))
    }
}
