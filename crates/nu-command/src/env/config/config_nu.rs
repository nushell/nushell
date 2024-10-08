use nu_cmd_base::util::get_editor;
use nu_engine::{command_prelude::*, env_to_strings};
use nu_protocol::{process::ChildProcess, ByteStream};
use nu_system::ForegroundChild;

#[derive(Clone)]
pub struct ConfigNu;

impl Command for ConfigNu {
    fn name(&self) -> &str {
        "config nu"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Env)
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .switch(
                "default",
                "Print default `config.nu` file instead.",
                Some('d'),
            )
        // TODO: Signature narrower than what run actually supports theoretically
    }

    fn description(&self) -> &str {
        "Edit nu configurations."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "allow user to open and update nu config",
                example: "config nu",
                result: None,
            },
            Example {
                description: "allow user to print default `config.nu` file",
                example: "config nu --default,",
                result: None,
            },
            Example {
                description: "allow saving the default `config.nu` locally",
                example: "config nu --default | save -f ~/.config/nushell/default_config.nu",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // `--default` flag handling
        if call.has_flag(engine_state, stack, "default")? {
            let head = call.head;
            return Ok(Value::string(nu_utils::get_default_config(), head).into_pipeline_data());
        }

        // Find the editor executable.
        let (editor_name, editor_args) = get_editor(engine_state, stack, call.head)?;
        let paths = nu_engine::env::path_str(engine_state, stack, call.head)?;
        let cwd = engine_state.cwd(Some(stack))?;
        let editor_executable = crate::which(&editor_name, &paths, cwd.as_ref()).ok_or(
            ShellError::ExternalCommand {
                label: format!("`{editor_name}` not found"),
                help: "Failed to find the editor executable".into(),
                span: call.head,
            },
        )?;

        let Some(config_path) = engine_state.get_config_path("config-path") else {
            return Err(ShellError::GenericError {
                error: "Could not find $nu.config-path".into(),
                msg: "Could not find $nu.config-path".into(),
                span: None,
                help: None,
                inner: vec![],
            });
        };
        let config_path = config_path.to_string_lossy().to_string();

        // Create the command.
        let mut command = std::process::Command::new(editor_executable);

        // Configure PWD.
        command.current_dir(cwd);

        // Configure environment variables.
        let envs = env_to_strings(engine_state, stack)?;
        command.env_clear();
        command.envs(envs);

        // Configure args.
        command.arg(config_path);
        command.args(editor_args);

        // Spawn the child process. On Unix, also put the child process to
        // foreground if we're in an interactive session.
        #[cfg(windows)]
        let child = ForegroundChild::spawn(command)?;
        #[cfg(unix)]
        let child = ForegroundChild::spawn(
            command,
            engine_state.is_interactive,
            &engine_state.pipeline_externals_state,
        )?;

        // Wrap the output into a `PipelineData::ByteStream`.
        let child = ChildProcess::new(child, None, false, call.head)?;
        Ok(PipelineData::ByteStream(
            ByteStream::child(child, call.head),
            None,
        ))
    }
}
