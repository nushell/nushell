use std::borrow::Cow;

use nu_engine::{command_prelude::*, env_to_strings};

#[derive(Clone)]
pub struct Exec;

impl Command for Exec {
    fn name(&self) -> &str {
        "exec"
    }

    fn signature(&self) -> Signature {
        Signature::build("exec")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .rest(
                "command",
                SyntaxShape::OneOf(vec![SyntaxShape::GlobPattern, SyntaxShape::Any]),
                "External command to run, with arguments.",
            )
            .allows_unknown_args()
            .category(Category::System)
    }

    fn description(&self) -> &str {
        "Execute a command, replacing or exiting the current process, depending on platform."
    }

    fn extra_description(&self) -> &str {
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
        let cwd = engine_state.cwd(Some(stack))?;
        let rest = call.rest::<Value>(engine_state, stack, 0)?;
        let name_args = rest.split_first();

        let Some((name, call_args)) = name_args else {
            return Err(ShellError::MissingParameter {
                param_name: "no command given".into(),
                span: call.head,
            });
        };

        let name_str: Cow<str> = match &name {
            Value::Glob { val, .. } => Cow::Borrowed(val),
            Value::String { val, .. } => Cow::Borrowed(val),
            _ => Cow::Owned(name.clone().coerce_into_string()?),
        };

        // Find the absolute path to the executable. If the command is not
        // found, display a helpful error message.
        let executable = {
            let paths = nu_engine::env::path_str(engine_state, stack, call.head)?;
            let Some(executable) = crate::which(name_str.as_ref(), &paths, cwd.as_ref()) else {
                return Err(crate::command_not_found(
                    &name_str,
                    call.head,
                    engine_state,
                    stack,
                    &cwd,
                ));
            };
            executable
        };

        // Create the command.
        let mut command = std::process::Command::new(executable);

        // Configure PWD.
        command.current_dir(cwd);

        // Configure environment variables.
        let envs = env_to_strings(engine_state, stack)?;
        command.env_clear();
        command.envs(envs);
        // Decrement SHLVL as removing the current shell from the stack
        // (only works in interactive mode, same as initialization)
        if engine_state.is_interactive {
            let shlvl = engine_state
                .get_env_var("SHLVL")
                .and_then(|shlvl_env| shlvl_env.coerce_str().ok()?.parse::<i64>().ok())
                .unwrap_or(1)
                .saturating_sub(1);
            command.env("SHLVL", shlvl.to_string());
        }

        // Configure args.
        let args = crate::eval_external_arguments(engine_state, stack, call_args.to_vec())?;
        command.args(args.into_iter().map(|s| s.item));

        // Execute the child process, replacing/terminating the current process
        // depending on platform.
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;

            let err = command.exec();
            Err(ShellError::ExternalCommand {
                label: "Failed to exec into new process".into(),
                help: err.to_string(),
                span: call.head,
            })
        }
        #[cfg(windows)]
        {
            let mut child = command.spawn().map_err(|err| ShellError::ExternalCommand {
                label: "Failed to exec into new process".into(),
                help: err.to_string(),
                span: call.head,
            })?;
            let status = child.wait().map_err(|err| ShellError::ExternalCommand {
                label: "Failed to wait for child process".into(),
                help: err.to_string(),
                span: call.head,
            })?;
            std::process::exit(status.code().expect("status.code() succeeds on Windows"))
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
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
