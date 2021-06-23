use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, Value};
use nu_test_support::{NATIVE_PATH_ENV_SEPARATOR, NATIVE_PATH_ENV_VAR};

pub struct Command;

impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "pathvar"
    }

    fn signature(&self) -> Signature {
        Signature::build("pathvar")
    }

    fn usage(&self) -> &str {
        "Manipulate the PATH variable (or pathvar)."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        get_pathvar(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Display the current session's pathvar",
                example: "pathvar",
                result: None,
            },
            Example {
                description: "Add /usr/bin to the pathvar",
                example: "pathvar add /usr/bin",
                result: None,
            },
            Example {
                description: "Remove the 3rd path in the pathvar",
                example: "pathvar remove 2",
                result: None,
            },
        ]
    }
}

pub fn get_pathvar(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if let Some(pathvar) = args.context.scope.get_env(NATIVE_PATH_ENV_VAR) {
        let pathvar: Vec<Value> = pathvar
            .split(NATIVE_PATH_ENV_SEPARATOR)
            .map(Value::from)
            .collect();

        Ok(OutputStream::from(pathvar))
    } else {
        Err(ShellError::unexpected("PATH not set"))
    }
}
