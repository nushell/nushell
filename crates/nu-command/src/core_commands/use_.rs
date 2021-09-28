use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{Signature, SyntaxShape, Value};

pub struct Use;

impl Command for Use {
    fn name(&self) -> &str {
        "use"
    }

    fn usage(&self) -> &str {
        "Use definitions from a module"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("use").required("pattern", SyntaxShape::String, "import pattern")
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
