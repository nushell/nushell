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

    fn usage(&self) -> &str {
        "Clear the terminal."
    }

    fn signature(&self) -> Signature {
        Signature::build("clear")
            .category(Category::Platform)
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .switch(
                "all",
                "Clear the terminal and its scroll-back history",
                Some('a'),
            )
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let clear_type: ClearType = match call.has_flag(engine_state, stack, "all")? {
            true => ClearType::Purge,
            _ => ClearType::All,
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
                description: "Clear the terminal and its scroll-back history",
                example: "clear --all",
                result: None,
            },
        ]
    }
}
