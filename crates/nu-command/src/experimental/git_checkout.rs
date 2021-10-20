use nu_engine::eval_expression;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{Signature, SyntaxShape, Value};

pub struct GitCheckout;

impl Command for GitCheckout {
    fn name(&self) -> &str {
        "git checkout"
    }

    fn usage(&self) -> &str {
        "Checkout a git revision"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("git checkout").required(
            "branch",
            SyntaxShape::Custom(Box::new(SyntaxShape::String), "list-git-branches".into()),
            "the branch to checkout",
        )
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        _input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        use std::process::Command as ProcessCommand;
        use std::process::Stdio;

        let block = &call.positional[0];

        let out = eval_expression(context, block)?;

        let out = out.as_string()?;

        let proc = ProcessCommand::new("git")
            .arg("checkout")
            .arg(out)
            .stdout(Stdio::piped())
            .spawn();

        match proc {
            Ok(child) => {
                match child.wait_with_output() {
                    Ok(val) => {
                        let result = val.stdout;

                        Ok(Value::String {
                            val: String::from_utf8_lossy(&result).to_string(),
                            span: call.head,
                        })
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
