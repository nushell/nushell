use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape,
};

#[derive(Clone)]
pub struct Exec;

impl Command for Exec {
    fn name(&self) -> &str {
        "exec"
    }

    fn signature(&self) -> Signature {
        Signature::build("exec")
            .required("command", SyntaxShape::String, "the command to execute")
            .rest(
                "rest",
                SyntaxShape::String,
                "any additional arguments for the command",
            )
            .category(Category::System)
    }

    fn usage(&self) -> &str {
        "Execute a command, replacing the current process."
    }

    fn extra_usage(&self) -> &str {
        "Currently supported only on Unix-based systems."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
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

#[cfg(unix)]
fn exec(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    use std::os::unix::process::CommandExt;

    use nu_engine::{current_dir, env_to_strings, CallExt};
    use nu_protocol::Spanned;

    use super::run_external::ExternalCommand;

    let name: Spanned<String> = call.req(engine_state, stack, 0)?;
    let name_span = name.span;

    let args: Vec<Spanned<String>> = call.rest(engine_state, stack, 1)?;

    let cwd = current_dir(engine_state, stack)?;
    let env_vars = env_to_strings(engine_state, stack)?;
    let current_dir = current_dir(engine_state, stack)?;

    let external_command = ExternalCommand {
        name,
        args,
        env_vars,
        redirect_stdout: true,
        redirect_stderr: false,
    };

    let mut command = external_command.spawn_simple_command(&cwd.to_string_lossy().to_string())?;
    command.current_dir(current_dir);

    println!("{:#?}", command);
    let err = command.exec(); // this replaces our process, should not return

    Err(ShellError::SpannedLabeledError(
        "Error on exec".to_string(),
        err.to_string(),
        name_span,
    ))
}

#[cfg(not(unix))]
fn exec(
    _engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    Err(ShellError::SpannedLabeledError(
        "Error on exec".to_string(),
        "exec is not supported on your platform".to_string(),
        call.head,
    ))
}
