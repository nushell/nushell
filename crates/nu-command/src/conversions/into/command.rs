use nu_engine::get_full_help;
use nu_protocol::{
    ast::Call,
    engine::{Command, EvaluationContext},
    Signature, Value,
};

pub struct Into;

impl Command for Into {
    fn name(&self) -> &str {
        "into"
    }

    fn signature(&self) -> Signature {
        Signature::build("into")
    }

    fn usage(&self) -> &str {
        "Apply into function."
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        _input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        Ok(Value::String {
            val: get_full_help(&Into.signature(), &[], context),
            span: call.head,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Into {})
    }
}
