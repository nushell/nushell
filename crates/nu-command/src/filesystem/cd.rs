use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{Signature, SyntaxShape, Value};

pub struct Cd;

impl Command for Cd {
    fn name(&self) -> &str {
        "cd"
    }

    fn usage(&self) -> &str {
        "Change directory."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("cd").optional("path", SyntaxShape::FilePath, "the path to change to")
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        _input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        let path: Option<String> = call.opt(context, 0)?;

        let path = match path {
            Some(path) => {
                let path = nu_path::expand_tilde(path);
                path.to_string_lossy().to_string()
            }
            None => {
                let path = nu_path::expand_tilde("~");
                path.to_string_lossy().to_string()
            }
        };
        let _ = std::env::set_current_dir(&path);

        //FIXME: this only changes the current scope, but instead this environment variable
        //should probably be a block that loads the information from the state in the overlay
        context.add_env_var("PWD".into(), path);
        Ok(Value::Nothing { span: call.head })
    }
}
