use std::{collections::HashMap, path::PathBuf};

use nu_protocol::{
    engine::{EngineState, Stack},
    Span, Spanned,
};

use crate::ExternalCommand;

pub(crate) fn get_editor(
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Result<(String, Vec<String>), nu_protocol::ShellError> {
    let config = engine_state.get_config();
    let env_vars = stack.get_env_vars(engine_state);
    let editor = if !config.buffer_editor.is_empty() {
        Ok(config.buffer_editor.clone())
    } else if let Some(value) = env_vars.get("EDITOR") {
        value.as_string()
    } else if let Some(value) = env_vars.get("VISUAL") {
        value.as_string()
    } else if cfg!(target_os = "windows") {
        Ok("notepad".to_string())
    } else {
        Ok("nano".to_string())
    }?;
    if let Some((a, b)) = editor.split_once(' ') {
        Ok((
            a.to_string(),
            b.split(' ')
                .into_iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
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
        env_vars: env_vars_str,
        trim_end_newline: false,
    }
}
