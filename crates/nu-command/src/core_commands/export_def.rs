use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{Signature, SyntaxShape, Value};

pub struct ExportDef;

impl Command for ExportDef {
    fn name(&self) -> &str {
        "export def"
    }

    fn usage(&self) -> &str {
        "Define a custom command and export it from a module"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("export def")
            .required("target", SyntaxShape::String, "definition name")
            .required("params", SyntaxShape::Signature, "parameters")
            .required(
                "block",
                SyntaxShape::Block(Some(vec![])),
                "body of the definition",
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
