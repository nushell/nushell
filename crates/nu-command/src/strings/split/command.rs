use nu_engine::get_full_help;
use nu_protocol::{
    ast::Call,
    engine::{Command, EvaluationContext},
    Signature, Value,
};

#[derive(Clone)]
pub struct SplitCommand;

impl Command for SplitCommand {
    fn name(&self) -> &str {
        "split"
    }

    fn signature(&self) -> Signature {
        Signature::build("split")
    }

    fn usage(&self) -> &str {
        "Split contents across desired subcommand (like row, column) via the separator."
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        _input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        Ok(Value::String {
            val: get_full_help(&SplitCommand.signature(), &SplitCommand.examples(), context),
            span: call.head,
        })
    }
}

// #[cfg(test)]
// mod tests {
//     use super::Command;
//     use super::ShellError;

//     #[test]
//     fn examples_work_as_expected() -> Result<(), ShellError> {
//         use crate::examples::test as test_examples;

//         test_examples(Command {})
//     }
// }
