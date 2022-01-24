use nu_engine::get_full_help;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, IntoPipelineData, PipelineData, Signature, Value,
};

#[derive(Clone)]
pub struct SplitCommand;

impl Command for SplitCommand {
    fn name(&self) -> &str {
        "split"
    }

    fn signature(&self) -> Signature {
        Signature::build("split").category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Split contents across desired subcommand (like row, column) via the separator."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(Value::String {
            val: get_full_help(
                &SplitCommand.signature(),
                &SplitCommand.examples(),
                engine_state,
                stack,
            ),
            span: call.head,
        }
        .into_pipeline_data())
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
