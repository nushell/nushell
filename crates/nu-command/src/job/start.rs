use std::io::IsTerminal;

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type,
};

use crate::ExternalCommand;

#[derive(Clone)]
pub struct JobStart;

impl Command for JobStart {
    fn name(&self) -> &str {
        "job start"
    }

    fn signature(&self) -> Signature {
        Signature::build("job start")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required(
                "command",
                SyntaxShape::String,
                "the external command to run",
            )
            .switch(
                "inherit",
                "make the background job inherit stdin, stdout, and stderr from the terminal",
                Some('i'),
            )
            .switch(
                "quiet",
                "in interactive mode, do not print a message when the job completes",
                Some('q'),
            )
            .rest(
                "args",
                SyntaxShape::Any,
                "the arguments for the external command",
            )
            .allows_unknown_args()
            .category(Category::Job)
    }

    fn usage(&self) -> &str {
        "Runs an external command in the background."
    }

    fn extra_usage(&self) -> &str {
        r"In non-interactive mode or when the --inherit flag is provided, the background job will inherit stdin, stdout, and stderr.
Otherwise, the background job will have separate input and output channels disconnected from the terminal."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let interactive = engine_state.is_interactive && std::io::stdin().is_terminal();
        let inherit_io = call.has_flag(engine_state, stack, "inherit")?;
        let quiet = call.has_flag(engine_state, stack, "quiet")?;

        let external = ExternalCommand::new(engine_state, stack, call, false, false, false, false)?;

        let head = external.name.span;
        let command = external.create_process(false, head)?;

        #[cfg(not(unix))]
        match engine_state
            .jobs
            .spawn_background(command, interactive, inherit_io, quiet)
        {
            Ok(id) => Ok(id),
            Err(err) => {
                #[cfg(not(windows))]
                {
                    Err(err)
                }

                #[cfg(windows)]
                {
                    if let Some(command) =
                        external.retry_command_windows(engine_state, stack, head)?
                    {
                        engine_state
                            .jobs
                            .spawn_background(command, interactive, inherit_io, quiet)
                    } else {
                        Err(err)
                    }
                }
            }
        }?;

        #[cfg(unix)]
        engine_state
            .jobs
            .spawn_background(command, interactive, inherit_io, quiet)?;

        Ok(PipelineData::Empty)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            // Example {
            //     description: "Run '' in the background",
            //     example: "job start ",
            //     result: None,
            // },
            // Example {
            //     description: "Run '' in the background",
            //     example: "job start ",
            //     result: None,
            // },
        ]
    }
}
