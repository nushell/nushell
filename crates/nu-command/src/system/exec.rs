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
            .required("command", SyntaxShape::String, "The command to execute.")
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

        // Find the absolute path to the executable. If the command is not
        // found, display a helpful error message.
        let name: Spanned<String> = call.req(engine_state, stack, 0)?;
        let executable = {
            let paths = nu_engine::env::path_str(engine_state, stack, call.head)?;
            let Some(executable) = crate::which(&name.item, &paths, cwd.as_ref()) else {
                return Err(crate::command_not_found(
                    &name.item,
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

        // Configure args.
        let args = crate::eval_arguments_from_call(engine_state, stack, call)?;
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
