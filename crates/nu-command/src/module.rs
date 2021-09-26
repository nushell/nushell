use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{Signature, SyntaxShape, Value};

pub struct Module;

impl Command for Module {
    fn name(&self) -> &str {
        "module"
    }

    fn usage(&self) -> &str {
        "Define a custom module"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("module")
            .required("module_name", SyntaxShape::String, "module name")
            .required(
                "block",
                SyntaxShape::Block(Some(vec![])),
                "body of the module",
            )
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
