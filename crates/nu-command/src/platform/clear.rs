use crossterm::{
    QueueableCommand,
    cursor::MoveTo,
    terminal::{Clear as ClearCommand, ClearType},
};
use nu_engine::command_prelude::*;
use nu_protocol::shell_error::io::IoError;

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
        let from_io_error = IoError::factory(call.head, None);
        match call.has_flag(engine_state, stack, "keep-scrollback")? {
            true => {
                std::io::stdout()
                    .queue(MoveTo(0, 0))
                    .map_err(&from_io_error)?
                    .queue(ClearCommand(ClearType::All))
                    .map_err(&from_io_error)?
                    .flush()
                    .map_err(&from_io_error)?;
            }
            _ => {
                std::io::stdout()
                    .queue(MoveTo(0, 0))
                    .map_err(&from_io_error)?
                    .queue(ClearCommand(ClearType::All))
                    .map_err(&from_io_error)?
                    .queue(ClearCommand(ClearType::Purge))
                    .map_err(&from_io_error)?
                    .flush()
                    .map_err(&from_io_error)?;
            }
        };

        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example<'_>> {
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
