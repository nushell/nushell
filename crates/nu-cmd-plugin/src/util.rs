#[allow(deprecated)]
use nu_engine::{command_prelude::*, current_dir};
use nu_protocol::{
    engine::{expand_path_with, StateWorkingSet},
    PluginRegistryFile,
};
use std::{
    fs::{self, File},
    path::PathBuf,
};

fn get_plugin_registry_file_path(
    engine_state: &EngineState,
    stack: &mut Stack,
    span: Span,
    custom_path: &Option<Spanned<String>>,
) -> Result<PathBuf, ShellError> {
    #[allow(deprecated)]
    let cwd = current_dir(engine_state, stack)?;

    if let Some(ref custom_path) = custom_path {
        Ok(expand_path_with(
            stack,
            engine_state,
            &custom_path.item,
            cwd,
            true,
        ))
    } else {
        engine_state
            .plugin_path
            .clone()
            .ok_or_else(|| ShellError::GenericError {
                error: "Plugin registry file not set".into(),
                msg: "pass --plugin-config explicitly here".into(),
                span: Some(span),
                help: Some("you may be running `nu` with --no-config-file".into()),
                inner: vec![],
            })
    }
}

pub(crate) fn read_plugin_file(
    engine_state: &EngineState,
    stack: &mut Stack,
    span: Span,
    custom_path: &Option<Spanned<String>>,
) -> Result<PluginRegistryFile, ShellError> {
    let plugin_registry_file_path =
        get_plugin_registry_file_path(engine_state, stack, span, custom_path)?;

    let file_span = custom_path.as_ref().map(|p| p.span).unwrap_or(span);

    // Try to read the plugin file if it exists
    if fs::metadata(&plugin_registry_file_path).is_ok_and(|m| m.len() > 0) {
        PluginRegistryFile::read_from(
            File::open(&plugin_registry_file_path).map_err(|err| ShellError::IOErrorSpanned {
                msg: format!(
                    "failed to read `{}`: {}",
                    plugin_registry_file_path.display(),
                    err
                ),
                span: file_span,
            })?,
            Some(file_span),
        )
    } else if let Some(path) = custom_path {
        Err(ShellError::FileNotFound {
            file: path.item.clone(),
            span: path.span,
        })
    } else {
        Ok(PluginRegistryFile::default())
    }
}

pub(crate) fn modify_plugin_file(
    engine_state: &EngineState,
    stack: &mut Stack,
    span: Span,
    custom_path: &Option<Spanned<String>>,
    operate: impl FnOnce(&mut PluginRegistryFile) -> Result<(), ShellError>,
) -> Result<(), ShellError> {
    let plugin_registry_file_path =
        get_plugin_registry_file_path(engine_state, stack, span, custom_path)?;

    let file_span = custom_path.as_ref().map(|p| p.span).unwrap_or(span);

    // Try to read the plugin file if it exists
    let mut contents = if fs::metadata(&plugin_registry_file_path).is_ok_and(|m| m.len() > 0) {
        PluginRegistryFile::read_from(
            File::open(&plugin_registry_file_path).map_err(|err| ShellError::IOErrorSpanned {
                msg: format!(
                    "failed to read `{}`: {}",
                    plugin_registry_file_path.display(),
                    err
                ),
                span: file_span,
            })?,
            Some(file_span),
        )?
    } else {
        PluginRegistryFile::default()
    };

    // Do the operation
    operate(&mut contents)?;

    // Save the modified file on success
    contents.write_to(
        File::create(&plugin_registry_file_path).map_err(|err| ShellError::IOErrorSpanned {
            msg: format!(
                "failed to create `{}`: {}",
                plugin_registry_file_path.display(),
                err
            ),
            span: file_span,
        })?,
        Some(span),
    )?;

    Ok(())
}

pub(crate) fn canonicalize_possible_filename_arg(
    engine_state: &EngineState,
    stack: &Stack,
    arg: &str,
) -> PathBuf {
    // This results in the best possible chance of a match with the plugin item
    #[allow(deprecated)]
    if let Ok(cwd) = nu_engine::current_dir(engine_state, stack) {
        let path = expand_path_with(stack, engine_state, arg, &cwd, true);
        // Try to canonicalize
        nu_path::locate_in_dirs(&path, &cwd, || get_plugin_dirs(engine_state, stack))
            // If we couldn't locate it, return the expanded path alone
            .unwrap_or(path)
    } else {
        arg.into()
    }
}

pub(crate) fn get_plugin_dirs(
    engine_state: &EngineState,
    stack: &Stack,
) -> impl Iterator<Item = String> {
    // Get the NU_PLUGIN_DIRS from the constant and/or env var
    let working_set = StateWorkingSet::new(engine_state);
    let dirs_from_const = working_set
        .find_variable(b"$NU_PLUGIN_DIRS")
        .and_then(|var_id| working_set.get_constant(var_id).ok())
        .cloned() // TODO: avoid this clone
        .into_iter()
        .flat_map(|value| value.into_list().ok())
        .flatten()
        .flat_map(|list_item| list_item.coerce_into_string().ok());

    let dirs_from_env = stack
        .get_env_var(engine_state, "NU_PLUGIN_DIRS")
        .cloned() // TODO: avoid this clone
        .into_iter()
        .flat_map(|value| value.into_list().ok())
        .flatten()
        .flat_map(|list_item| list_item.coerce_into_string().ok());

    dirs_from_const.chain(dirs_from_env)
}
