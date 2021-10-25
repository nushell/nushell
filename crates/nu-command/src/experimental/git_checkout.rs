use nu_engine::eval_expression;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, EvaluationContext, Stack};
use nu_protocol::{IntoPipelineData, PipelineData, Signature, SyntaxShape, Value};

#[derive(Clone)]
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
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        use std::process::Command as ProcessCommand;
        use std::process::Stdio;

        let block = &call.positional[0];

        let out = eval_expression(engine_state, stack, block)?;

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
                        }
                        .into_pipeline_data())
                    }
                    Err(_err) => {
                        // FIXME: Move this to an external signature and add better error handling
                        Ok(PipelineData::new())
                    }
                }
            }
            Err(_err) => {
                // FIXME: Move this to an external signature and add better error handling
                Ok(PipelineData::new())
            }
        }
    }
}
