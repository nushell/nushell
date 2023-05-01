use super::run_external::create_external_command;
use nu_engine::{current_dir, CallExt};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type,
};
use std::os::unix::process::CommandExt;

#[derive(Clone)]
pub struct Exec;

impl Command for Exec {
    fn name(&self) -> &str {
        "exec"
    }

    fn signature(&self) -> Signature {
        Signature::build("exec")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .required("command", SyntaxShape::String, "the command to execute")
            .allows_unknown_args()
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
    let name: Spanned<String> = call.req(engine_state, stack, 0)?;
    let name_span = name.span;

    let redirect_stdout = call.has_flag("redirect-stdout");
    let redirect_stderr = call.has_flag("redirect-stderr");
    let redirect_combine = call.has_flag("redirect-combine");
    let trim_end_newline = call.has_flag("trim-end-newline");

    let external_command = create_external_command(
        engine_state,
        stack,
        call,
        redirect_stdout,
        redirect_stderr,
        redirect_combine,
        trim_end_newline,
    )?;

    let cwd = current_dir(engine_state, stack)?;
    let mut command = external_command.spawn_simple_command(&cwd.to_string_lossy())?;
    command.current_dir(cwd);
    command.envs(&external_command.env_vars);

    let err = command.exec(); // this replaces our process, should not return

    Err(ShellError::GenericError(
        "Error on exec".to_string(),
        err.to_string(),
        Some(name_span),
        None,
        Vec::new(),
    ))
}
