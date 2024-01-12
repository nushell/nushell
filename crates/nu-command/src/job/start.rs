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
        ""
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let external = ExternalCommand::new(engine_state, stack, call, false, false, false, false)?;

        let head = external.name.span;
        let command = external.create_process(false, head)?;

        #[cfg(not(unix))]
        match engine_state
            .jobs
            .spawn_background(command, engine_state.is_interactive)
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
                            .spawn_background(command, engine_state.is_interactive)
                    } else {
                        Err(err)
                    }
                }
            }
        }?;

        #[cfg(unix)]
        engine_state
            .jobs
            .spawn_background(command, engine_state.is_interactive)?;

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
