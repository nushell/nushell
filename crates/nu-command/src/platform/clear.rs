use crossterm::{
    cursor::MoveTo,
    terminal::{Clear as ClearCommand, ClearType},
    QueueableCommand,
};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Type};
use std::io::Write;

#[derive(Clone)]
pub struct Clear;

impl Command for Clear {
    fn name(&self) -> &str {
        "clear"
    }

    fn usage(&self) -> &str {
        "Clear the terminal."
    }

    fn signature(&self) -> Signature {
        Signature::build("clear")
            .category(Category::Platform)
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        std::io::stdout()
            .queue(ClearCommand(ClearType::All))?
            .queue(MoveTo(0, 0))?
            .flush()?;

        Ok(PipelineData::Empty)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Clear the terminal",
            example: "clear",
            result: None,
        }]
    }
}
