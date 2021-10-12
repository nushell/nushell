use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{Signature, Value};

pub struct Git;

impl Command for Git {
    fn name(&self) -> &str {
        "git"
    }

    fn usage(&self) -> &str {
        "Run a block"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("git")
    }

    fn run(
        &self,
        _context: &EvaluationContext,
        call: &Call,
        _input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        use std::process::Command as ProcessCommand;
        use std::process::Stdio;

        let proc = ProcessCommand::new("git").stdout(Stdio::piped()).spawn();

        match proc {
            Ok(child) => {
                match child.wait_with_output() {
                    Ok(val) => {
                        let result = val.stdout;

                        Ok(Value::string(&String::from_utf8_lossy(&result), call.head))
                    }
                    Err(_err) => {
                        // FIXME: Move this to an external signature and add better error handling
                        Ok(Value::nothing())
                    }
                }
            }
            Err(_err) => {
                // FIXME: Move this to an external signature and add better error handling
                Ok(Value::nothing())
            }
        }
    }
}
