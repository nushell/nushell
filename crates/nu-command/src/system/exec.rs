use super::run_external::ExternalCommand;
use nu_engine::{current_dir, env_to_strings, CallExt};
use nu_protocol::{
    ast::{Call, Expr},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape,
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

fn exec(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let name: Spanned<String> = call.req(engine_state, stack, 0)?;
    let name_span = name.span;

    let args: Vec<Spanned<String>> = call.rest(engine_state, stack, 1)?;
    let args_expr: Vec<nu_protocol::ast::Expression> =
        call.positional_iter().skip(1).cloned().collect();
    let mut arg_keep_raw = vec![];
    for one_arg_expr in args_expr {
        match one_arg_expr.expr {
            // refer to `parse_dollar_expr` function
            // the expression type of $variable_name, $"($variable_name)"
            // will be Expr::StringInterpolation, Expr::FullCellPath
            Expr::StringInterpolation(_) | Expr::FullCellPath(_) => arg_keep_raw.push(true),
            _ => arg_keep_raw.push(false),
        }
    }

    let cwd = current_dir(engine_state, stack)?;
    let env_vars = env_to_strings(engine_state, stack)?;
    let current_dir = current_dir(engine_state, stack)?;

    let external_command = ExternalCommand {
        name,
        args,
        arg_keep_raw,
        env_vars,
        redirect_stdout: true,
        redirect_stderr: false,
    };

    let mut command = external_command.spawn_simple_command(&cwd.to_string_lossy())?;
    command.current_dir(current_dir);

    let err = command.exec(); // this replaces our process, should not return

    Err(ShellError::GenericError(
        "Error on exec".to_string(),
        err.to_string(),
        Some(name_span),
        None,
        Vec::new(),
    ))
}
