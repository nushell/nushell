use crate::prelude::*;
use nu_engine::WholeStreamCommand;

use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;

pub struct UnletEnv;

#[derive(Deserialize)]
pub struct UnletEnvArgs {
    pub name: Tagged<String>,
}

impl WholeStreamCommand for UnletEnv {
    fn name(&self) -> &str {
        "unlet-env"
    }

    fn signature(&self) -> Signature {
        Signature::build("unlet-env").required(
            "name",
            SyntaxShape::String,
            "the name of the environment variable",
        )
    }

    fn usage(&self) -> &str {
        "Delete an environment variable."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        unlet_env(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Remove the environment variable named FOO.",
            example: "unlet-env FOO",
            result: None,
        }]
    }
}

pub fn unlet_env(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let ctx = &args.context;

    let name: Tagged<String> = args.req(0)?;

    if ctx.scope.remove_env_var(&name.item) == None {
        return Err(ShellError::labeled_error(
            "Not an environment variable. Run `echo $nu.env` to view the available variables.",
            "not an environment variable",
            name.span(),
        ));
    }

    Ok(ActionStream::empty())
}
