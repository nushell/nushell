use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{ShellError, Signature, Value};

pub struct From;

impl Command for From {
    fn name(&self) -> &str {
        "from"
    }

    fn usage(&self) -> &str {
        "Parse a string or binary data into structured data"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("from")
    }

    fn run(
        &self,
        _context: &EvaluationContext,
        _call: &Call,
        _input: Value,
    ) -> Result<nu_protocol::Value, ShellError> {
        Ok(Value::nothing())
    }
}
