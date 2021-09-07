use nu_engine::eval_expression;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{Signature, SyntaxShape, Value};

pub struct BuildString;

impl Command for BuildString {
    fn name(&self) -> &str {
        "build-string"
    }

    fn usage(&self) -> &str {
        "Create a string from the arguments."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("build-string").rest("rest", SyntaxShape::String, "list of string")
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        _input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        let mut output = vec![];

        for expr in &call.positional {
            let val = eval_expression(context, expr)?;

            output.push(val.into_string());
        }
        Ok(Value::String {
            val: output.join(""),
            span: call.head,
        })
    }
}
