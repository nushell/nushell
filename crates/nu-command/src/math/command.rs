use nu_engine::get_full_help;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{ShellError, Signature, Value};

pub struct MathCommand;

impl Command for MathCommand {
    fn name(&self) -> &str {
        "math"
    }

    fn signature(&self) -> Signature {
        Signature::build("math")
    }

    fn usage(&self) -> &str {
        "Use mathematical functions as aggregate functions on a list of numbers or tables."
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        _input: Value,
    ) -> Result<Value, ShellError> {
        Ok(Value::String {
            val: get_full_help(&MathCommand.signature(), &MathCommand.examples(), context),
            span: call.head,
        })
    }
}
