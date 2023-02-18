use nu_protocol::engine::{EngineState, Stack};

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
    if let Some((a, b)) = editor.split_once(" ") {
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
