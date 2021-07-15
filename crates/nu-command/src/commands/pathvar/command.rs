use super::get_var;
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};
use nu_test_support::NATIVE_PATH_ENV_SEPARATOR;

pub struct Command;

impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "pathvar"
    }

    fn signature(&self) -> Signature {
        Signature::build("pathvar").named(
            "var",
            SyntaxShape::String,
            "Use a different variable than PATH",
            Some('v'),
        )
    }

    fn usage(&self) -> &str {
        r#"Manipulate the PATH variable (pathvar) or a different variable following the
same rules."#
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
                description: "Display the current session's LD_LIBRARY_PATH",
                example: "pathvar -v LD_LIBRARY_PATH",
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
    let var = get_var(&args)?;

    if let Some(pathvar) = args.context.scope.get_env(&var) {
        let pathvar: Vec<Value> = pathvar
            .split(NATIVE_PATH_ENV_SEPARATOR)
            .map(Value::from)
            .collect();

        Ok(OutputStream::from(pathvar))
    } else {
        Err(ShellError::unexpected(&format!(
            "Variable {} not set",
            &var.item
        )))
    }
}
