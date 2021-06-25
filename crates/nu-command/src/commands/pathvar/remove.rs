use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;
use nu_test_support::{NATIVE_PATH_ENV_SEPARATOR, NATIVE_PATH_ENV_VAR};

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "pathvar remove"
    }

    fn signature(&self) -> Signature {
        Signature::build("pathvar remove").required(
            "index",
            SyntaxShape::Int,
            "index of the path to remove (starting at 0)",
        )
    }

    fn usage(&self) -> &str {
        "Remove a path from the pathvar"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        remove(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Remove the second path from the pathvar",
            example: "pathvar remove 1",
            result: None,
        }]
    }
}

pub fn remove(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctx = &args.context;
    let index_to_remove_arg: Tagged<u64> = args.req(0)?;
    let index_to_remove = index_to_remove_arg.item as usize;

    if let Some(old_pathvar) = ctx.scope.get_env(NATIVE_PATH_ENV_VAR) {
        let mut paths: Vec<&str> = old_pathvar.split(NATIVE_PATH_ENV_SEPARATOR).collect();

        if index_to_remove >= paths.len() {
            return Err(ShellError::labeled_error(
                "Index out of bounds",
                format!("the index must be between 0 and {}", paths.len() - 1),
                index_to_remove_arg.tag,
            ));
        }

        paths.remove(index_to_remove);
        ctx.scope.add_env_var(
            NATIVE_PATH_ENV_VAR,
            paths.join(&NATIVE_PATH_ENV_SEPARATOR.to_string()),
        );

        Ok(OutputStream::empty())
    } else {
        Err(ShellError::unexpected("PATH not set"))
    }
}
