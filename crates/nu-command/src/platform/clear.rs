use crossterm::{
    cursor::MoveTo,
    terminal::{Clear as ClearCommand, ClearType},
    QueueableCommand,
};
use nu_engine::command_prelude::*;

use std::io::Write;

#[derive(Clone)]
pub struct Clear;

impl Command for Clear {
    fn name(&self) -> &str {
        "clear"
    }

    fn description(&self) -> &str {
        "Clear the terminal."
    }

    fn extra_description(&self) -> &str {
        "By default clears the current screen and the off-screen scrollback buffer."
    }

    fn signature(&self) -> Signature {
        Signature::build("clear")
            .category(Category::Platform)
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .switch(
                "keep-scrollback",
                "Do not clear the scrollback history",
                Some('k'),
            )
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let clear_type: ClearType = match call.has_flag(engine_state, stack, "keep-scrollback")? {
            true => ClearType::All,
            _ => ClearType::Purge,
        };
        std::io::stdout()
            .queue(ClearCommand(clear_type))?
            .queue(MoveTo(0, 0))?
            .flush()?;

        Ok(PipelineData::Empty)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Clear the terminal",
                example: "clear",
                result: None,
            },
            Example {
                description: "Clear the terminal but not its scrollback history",
                example: "clear --keep-scrollback",
                result: None,
            },
        ]
    }
}
