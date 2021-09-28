use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{Signature, SyntaxShape, Value};

pub struct Hide;

impl Command for Hide {
    fn name(&self) -> &str {
        "hide"
    }

    fn usage(&self) -> &str {
        "Hide definitions in the current scope"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("hide").required("pattern", SyntaxShape::String, "import pattern")
    }

    fn run(
        &self,
        _context: &EvaluationContext,
        call: &Call,
        _input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        Ok(Value::Nothing { span: call.head })
    }
}
