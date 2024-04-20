use std::fs::{self, File};

use nu_engine::{command_prelude::*, current_dir};
use nu_protocol::PluginCacheFile;

pub(crate) fn modify_plugin_file(
    engine_state: &EngineState,
    stack: &mut Stack,
    span: Span,
    custom_path: Option<Spanned<String>>,
    operate: impl FnOnce(&mut PluginCacheFile) -> Result<(), ShellError>,
) -> Result<(), ShellError> {
    let cwd = current_dir(engine_state, stack)?;

    let plugin_cache_file_path = if let Some(ref custom_path) = custom_path {
        nu_path::expand_path_with(&custom_path.item, cwd, true)
    } else {
        engine_state
            .plugin_path
            .clone()
            .ok_or_else(|| ShellError::GenericError {
                error: "Plugin cache file not set".into(),
                msg: "pass --plugin-config explicitly here".into(),
                span: Some(span),
                help: Some("you may be running `nu` with --no-config-file".into()),
                inner: vec![],
            })?
    };

    // Try to read the plugin file if it exists
    let mut contents = if fs::metadata(&plugin_cache_file_path).is_ok_and(|m| m.len() > 0) {
        PluginCacheFile::read_from(
            File::open(&plugin_cache_file_path).map_err(|err| err.into_spanned(span))?,
            Some(span),
        )?
    } else {
        PluginCacheFile::default()
    };

    // Do the operation
    operate(&mut contents)?;

    // Save the modified file on success
    contents.write_to(
        File::create(&plugin_cache_file_path).map_err(|err| err.into_spanned(span))?,
        Some(span),
    )?;

    Ok(())
}
