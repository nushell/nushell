use super::run_external::create_external_command;
use nu_engine::{command_prelude::*, current_dir};
use nu_protocol::OutDest;

#[derive(Clone)]
pub struct Exec;

impl Command for Exec {
    fn name(&self) -> &str {
        "exec"
    }

    fn signature(&self) -> Signature {
        Signature::build("exec")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .required("command", SyntaxShape::String, "The command to execute.")
            .allows_unknown_args()
            .category(Category::System)
    }

    fn usage(&self) -> &str {
        "Execute a command, replacing or exiting the current process, depending on platform."
    }

    fn extra_usage(&self) -> &str {
        r#"On Unix-based systems, the current process is replaced with the command.
On Windows based systems, Nushell will wait for the command to finish and then exit with the command's exit code."#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        exec(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Execute external 'ps aux' tool",
                example: "exec ps aux",
                result: None,
            },
            Example {
                description: "Execute 'nautilus'",
                example: "exec nautilus",
                result: None,
            },
        ]
    }
}

fn exec(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let mut external_command = create_external_command(engine_state, stack, call)?;
    external_command.out = OutDest::Inherit;
    external_command.err = OutDest::Inherit;

    let cwd = current_dir(engine_state, stack)?;
    let mut command = external_command.spawn_simple_command(&cwd.to_string_lossy())?;
    command.current_dir(cwd);
    command.envs(external_command.env_vars);

    // this either replaces our process and should not return,
    // or the exec fails and we get an error back
    exec_impl(command, call.head)
}

#[cfg(unix)]
fn exec_impl(mut command: std::process::Command, span: Span) -> Result<PipelineData, ShellError> {
    use std::os::unix::process::CommandExt;

    let error = command.exec();

    Err(ShellError::GenericError {
        error: "Error on exec".into(),
        msg: error.to_string(),
        span: Some(span),
        help: None,
        inner: vec![],
    })
}

#[cfg(windows)]
fn exec_impl(mut command: std::process::Command, span: Span) -> Result<PipelineData, ShellError> {
    match command.spawn() {
        Ok(mut child) => match child.wait() {
            Ok(status) => std::process::exit(status.code().unwrap_or(0)),
            Err(e) => Err(ShellError::ExternalCommand {
                label: "Error in external command".into(),
                help: e.to_string(),
                span,
            }),
        },
        Err(e) => Err(ShellError::ExternalCommand {
            label: "Error spawning external command".into(),
            help: e.to_string(),
            span,
        }),
    }
}
