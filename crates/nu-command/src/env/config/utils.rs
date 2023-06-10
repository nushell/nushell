use ahash::HashMap;
use std::path::PathBuf;

use nu_protocol::{
    engine::{EngineState, Stack},
    ShellError, Span, Spanned,
};

use crate::ExternalCommand;

pub(crate) fn get_editor(
    engine_state: &EngineState,
    stack: &mut Stack,
    span: Span,
) -> Result<(String, Vec<String>), ShellError> {
    let config = engine_state.get_config();
    let env_vars = stack.get_env_vars(engine_state);
    let editor = if !config.buffer_editor.is_empty() {
        Ok(config.buffer_editor.clone())
    } else if let Some(value) = env_vars.get("EDITOR") {
        value.as_string()
    } else if let Some(value) = env_vars.get("VISUAL") {
        value.as_string()
    } else {
        Err(ShellError::GenericError(
            "No editor configured".into(),
            "Please specify one via environment variables $EDITOR or $VISUAL".into(),
            Some(span),
            Some(
                "Nushell's config file can be found with the command: $nu.config-path. For more help: (https://nushell.sh/book/configuration.html#configurations-with-built-in-commands)"
                    .into(),
            ),
            vec![],
        ))
    }?;
    if let Some((a, b)) = editor.split_once(' ') {
        Ok((
            a.to_string(),
            b.split(' ').map(|s| s.to_string()).collect::<Vec<String>>(),
        ))
    } else {
        Ok((editor, Vec::new()))
    }
}

pub(crate) fn gen_command(
    span: Span,
    config_path: PathBuf,
    item: String,
    config_args: Vec<String>,
    env_vars_str: HashMap<String, String>,
) -> ExternalCommand {
    let name = Spanned { item, span };

    let mut args = vec![Spanned {
        item: config_path.to_string_lossy().to_string(),
        span: Span::unknown(),
    }];

    let number_of_args = config_args.len() + 1;

    for arg in config_args {
        args.push(Spanned {
            item: arg,
            span: Span::unknown(),
        })
    }

    ExternalCommand {
        name,
        args,
        arg_keep_raw: vec![false; number_of_args],
        redirect_stdout: false,
        redirect_stderr: false,
        redirect_combine: false,
        env_vars: env_vars_str,
        trim_end_newline: false,
    }
}
