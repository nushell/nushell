use nu_cmd_base::util::get_editor;
use nu_engine::{command_prelude::*, env_to_strings, get_full_help};
use nu_protocol::{PipelineMetadata, shell_error::io::IoError};
use nu_system::ForegroundChild;
use nu_utils::ConfigFileKind;

#[cfg(feature = "os")]
use nu_protocol::process::PostWaitCallback;

#[derive(Clone)]
pub struct ConfigMeta;

impl Command for ConfigMeta {
    fn name(&self) -> &str {
        "config"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Env)
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn description(&self) -> &str {
        "Edit nushell configuration files."
    }

    fn extra_description(&self) -> &str {
        "You must use one of the following subcommands. Using this command as-is will only produce this help message."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::string(get_full_help(self, engine_state, stack), call.head).into_pipeline_data())
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["options", "setup"]
    }
}

#[cfg(not(feature = "os"))]
pub(super) fn start_editor(
    _: ConfigFileKind,
    _: &EngineState,
    _: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    Err(ShellError::DisabledOsSupport {
        msg: "Running external commands is not available without OS support.".to_string(),
        span: call.head,
    })
}

#[cfg(feature = "os")]
pub(super) fn start_editor(
    kind: ConfigFileKind,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    // Find the editor executable.

    let (editor_name, editor_args) = get_editor(engine_state, stack, call.head)?;
    let paths = nu_engine::env::path_str(engine_state, stack, call.head)?;
    let cwd = engine_state.cwd(Some(stack))?;
    let editor_executable =
        crate::which(&editor_name, &paths, cwd.as_ref()).ok_or(ShellError::ExternalCommand {
            label: format!("`{editor_name}` not found"),
            help: "Failed to find the editor executable".into(),
            span: call.head,
        })?;

    let nu_const_path = kind.nu_const_path();
    let Some(config_path) = engine_state.get_config_path(nu_const_path) else {
        return Err(ShellError::GenericError {
            error: format!("Could not find $nu.{nu_const_path}"),
            msg: format!("Could not find $nu.{nu_const_path}"),
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
    let child = ForegroundChild::spawn(command);
    #[cfg(unix)]
    let child = ForegroundChild::spawn(
        command,
        engine_state.is_interactive,
        engine_state.is_background_job(),
        &engine_state.pipeline_externals_state,
    );

    let child = child.map_err(|err| {
        IoError::new_with_additional_context(
            err,
            call.head,
            None,
            "Could not spawn foreground child",
        )
    })?;

    let post_wait_callback = PostWaitCallback::for_job_control(engine_state, None, None);

    // Wrap the output into a `PipelineData::byte_stream`.
    let child = nu_protocol::process::ChildProcess::new(
        child,
        None,
        false,
        call.head,
        Some(post_wait_callback),
    )?;

    Ok(PipelineData::byte_stream(
        ByteStream::child(child, call.head),
        None,
    ))
}

pub(super) fn handle_call(
    kind: ConfigFileKind,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let default_flag = call.has_flag(engine_state, stack, "default")?;
    let doc_flag = call.has_flag(engine_state, stack, "doc")?;

    Ok(match (default_flag, doc_flag) {
        (false, false) => {
            return super::config_::start_editor(kind, engine_state, stack, call);
        }
        (true, true) => {
            return Err(ShellError::IncompatibleParameters {
                left_message: "can't use `--default` at the same time".into(),
                left_span: call.get_flag_span(stack, "default").expect("has flag"),
                right_message: "because of `--doc`".into(),
                right_span: call.get_flag_span(stack, "doc").expect("has flag"),
            });
        }
        (true, false) => kind.default(),
        (false, true) => kind.doc(),
    }
    .into_value(call.head)
    .into_pipeline_data_with_metadata(
        PipelineMetadata::default().with_content_type(Some("application/x-nuscript".into())),
    ))
}
