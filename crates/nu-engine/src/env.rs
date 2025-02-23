use crate::ClosureEvalOnce;
use nu_path::canonicalize_with;
use nu_protocol::{
    ast::Expr,
    engine::{Call, EngineState, Stack, StateWorkingSet},
    shell_error::io::{ErrorKindExt, IoError, NotFound},
    ShellError, Span, Type, Value, VarId,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

pub const ENV_CONVERSIONS: &str = "ENV_CONVERSIONS";

enum ConversionError {
    ShellError(ShellError),
    CellPathError,
}

impl From<ShellError> for ConversionError {
    fn from(value: ShellError) -> Self {
        Self::ShellError(value)
    }
}

/// Translate environment variables from Strings to Values.
pub fn convert_env_vars(
    stack: &mut Stack,
    engine_state: &EngineState,
    conversions: &Value,
) -> Result<(), ShellError> {
    let conversions = conversions.as_record()?;
    for (key, conversion) in conversions.into_iter() {
        if let Some((case_preserve_env_name, val)) =
            stack.get_env_var_insensitive(engine_state, key)
        {
            match val.get_type() {
                Type::String => {}
                _ => continue,
            }

            let conversion = conversion
                .as_record()?
                .get("from_string")
                .ok_or(ShellError::MissingRequiredColumn {
                    column: "from_string",
                    span: conversion.span(),
                })?
                .as_closure()?;

            let new_val = ClosureEvalOnce::new(engine_state, stack, conversion.clone())
                .debug(false)
                .run_with_value(val.clone())?
                .into_value(val.span())?;

            stack.add_env_var(case_preserve_env_name.to_string(), new_val);
        }
    }
    Ok(())
}

/// Translate environment variables from Strings to Values. Requires config to be already set up in
/// case the user defined custom env conversions in config.nu.
///
/// It returns Option instead of Result since we do want to translate all the values we can and
/// skip errors. This function is called in the main() so we want to keep running, we cannot just
/// exit.
pub fn convert_env_values(
    engine_state: &mut EngineState,
    stack: &mut Stack,
) -> Result<(), ShellError> {
    let mut error = None;

    let mut new_scope = HashMap::new();

    let env_vars = engine_state.render_env_vars();

    for (name, val) in env_vars {
        if let Value::String { .. } = val {
            // Only run from_string on string values
            match get_converted_value(engine_state, stack, name, val, "from_string") {
                Ok(v) => {
                    let _ = new_scope.insert(name.to_string(), v);
                }
                Err(ConversionError::ShellError(e)) => error = error.or(Some(e)),
                Err(ConversionError::CellPathError) => {
                    let _ = new_scope.insert(name.to_string(), val.clone());
                }
            }
        } else {
            // Skip values that are already converted (not a string)
            let _ = new_scope.insert(name.to_string(), val.clone());
        }
    }

    error = error.or_else(|| ensure_path(engine_state, stack));

    if let Ok(last_overlay_name) = &stack.last_overlay_name() {
        if let Some(env_vars) = Arc::make_mut(&mut engine_state.env_vars).get_mut(last_overlay_name)
        {
            for (k, v) in new_scope {
                env_vars.insert(k, v);
            }
        } else {
            error = error.or_else(|| {
                Some(ShellError::NushellFailedHelp { msg: "Last active overlay not found in permanent state.".into(), help: "This error happened during the conversion of environment variables from strings to Nushell values.".into() })
            });
        }
    } else {
        error = error.or_else(|| {
            Some(ShellError::NushellFailedHelp { msg: "Last active overlay not found in stack.".into(), help: "This error happened during the conversion of environment variables from strings to Nushell values.".into() })
        });
    }

    if let Some(err) = error {
        Err(err)
    } else {
        Ok(())
    }
}

/// Translate one environment variable from Value to String
///
/// Returns Ok(None) if the env var is not
pub fn env_to_string(
    env_name: &str,
    value: &Value,
    engine_state: &EngineState,
    stack: &Stack,
) -> Result<String, ShellError> {
    match get_converted_value(engine_state, stack, env_name, value, "to_string") {
        Ok(v) => Ok(v.coerce_into_string()?),
        Err(ConversionError::ShellError(e)) => Err(e),
        Err(ConversionError::CellPathError) => match value.coerce_string() {
            Ok(s) => Ok(s),
            Err(_) => {
                if env_name.to_lowercase() == "path" {
                    // Try to convert PATH/Path list to a string
                    match value {
                        Value::List { vals, .. } => {
                            let paths: Vec<String> = vals
                                .iter()
                                .filter_map(|v| v.coerce_str().ok())
                                .map(|s| nu_path::expand_tilde(&*s).to_string_lossy().into_owned())
                                .collect();

                            std::env::join_paths(paths.iter().map(AsRef::<str>::as_ref))
                                .map(|p| p.to_string_lossy().to_string())
                                .map_err(|_| ShellError::EnvVarNotAString {
                                    envvar_name: env_name.to_string(),
                                    span: value.span(),
                                })
                        }
                        _ => Err(ShellError::EnvVarNotAString {
                            envvar_name: env_name.to_string(),
                            span: value.span(),
                        }),
                    }
                } else {
                    Err(ShellError::EnvVarNotAString {
                        envvar_name: env_name.to_string(),
                        span: value.span(),
                    })
                }
            }
        },
    }
}

/// Translate all environment variables from Values to Strings
pub fn env_to_strings(
    engine_state: &EngineState,
    stack: &Stack,
) -> Result<HashMap<String, String>, ShellError> {
    let env_vars = stack.get_env_vars(engine_state);
    let mut env_vars_str = HashMap::new();
    for (env_name, val) in env_vars {
        match env_to_string(&env_name, &val, engine_state, stack) {
            Ok(val_str) => {
                env_vars_str.insert(env_name, val_str);
            }
            Err(ShellError::EnvVarNotAString { .. }) => {} // ignore non-string values
            Err(e) => return Err(e),
        }
    }

    Ok(env_vars_str)
}

/// Returns the current working directory as a String, which is guaranteed to be canonicalized.
/// Unlike `current_dir_str_const()`, this also considers modifications to the current working directory made on the stack.
///
/// Returns an error if $env.PWD doesn't exist, is not a String, or is not an absolute path.
#[deprecated(since = "0.92.3", note = "please use `EngineState::cwd()` instead")]
pub fn current_dir_str(engine_state: &EngineState, stack: &Stack) -> Result<String, ShellError> {
    #[allow(deprecated)]
    current_dir(engine_state, stack).map(|path| path.to_string_lossy().to_string())
}

/// Returns the current working directory as a String, which is guaranteed to be canonicalized.
///
/// Returns an error if $env.PWD doesn't exist, is not a String, or is not an absolute path.
#[deprecated(since = "0.92.3", note = "please use `EngineState::cwd()` instead")]
pub fn current_dir_str_const(working_set: &StateWorkingSet) -> Result<String, ShellError> {
    #[allow(deprecated)]
    current_dir_const(working_set).map(|path| path.to_string_lossy().to_string())
}

/// Returns the current working directory, which is guaranteed to be canonicalized.
/// Unlike `current_dir_const()`, this also considers modifications to the current working directory made on the stack.
///
/// Returns an error if $env.PWD doesn't exist, is not a String, or is not an absolute path.
#[deprecated(since = "0.92.3", note = "please use `EngineState::cwd()` instead")]
pub fn current_dir(engine_state: &EngineState, stack: &Stack) -> Result<PathBuf, ShellError> {
    let cwd = engine_state.cwd(Some(stack))?;
    // `EngineState::cwd()` always returns absolute path.
    // We're using `canonicalize_with` instead of `fs::canonicalize()` because
    // we still need to simplify Windows paths. "." is safe because `cwd` should
    // be an absolute path already.
    canonicalize_with(&cwd, ".").map_err(|err| {
        ShellError::Io(IoError::new_internal_with_path(
            err.kind().not_found_as(NotFound::Directory),
            "Could not canonicalize current dir",
            nu_protocol::location!(),
            PathBuf::from(cwd),
        ))
    })
}

/// Returns the current working directory, which is guaranteed to be canonicalized.
///
/// Returns an error if $env.PWD doesn't exist, is not a String, or is not an absolute path.
#[deprecated(since = "0.92.3", note = "please use `EngineState::cwd()` instead")]
pub fn current_dir_const(working_set: &StateWorkingSet) -> Result<PathBuf, ShellError> {
    let cwd = working_set.permanent_state.cwd(None)?;
    // `EngineState::cwd()` always returns absolute path.
    // We're using `canonicalize_with` instead of `fs::canonicalize()` because
    // we still need to simplify Windows paths. "." is safe because `cwd` should
    // be an absolute path already.
    canonicalize_with(&cwd, ".").map_err(|err| {
        ShellError::Io(IoError::new_internal_with_path(
            err.kind().not_found_as(NotFound::Directory),
            "Could not canonicalize current dir",
            nu_protocol::location!(),
            PathBuf::from(cwd),
        ))
    })
}

/// Get the contents of path environment variable as a list of strings
pub fn path_str(
    engine_state: &EngineState,
    stack: &Stack,
    span: Span,
) -> Result<String, ShellError> {
    let (pathname, pathval) = match stack.get_env_var_insensitive(engine_state, "path") {
        Some((_, v)) => Ok((if cfg!(windows) { "Path" } else { "PATH" }, v)),
        None => Err(ShellError::EnvVarNotFoundAtRuntime {
            envvar_name: if cfg!(windows) {
                "Path".to_string()
            } else {
                "PATH".to_string()
            },
            span,
        }),
    }?;

    env_to_string(pathname, pathval, engine_state, stack)
}

pub const DIR_VAR_PARSER_INFO: &str = "dirs_var";
pub fn get_dirs_var_from_call(stack: &Stack, call: &Call) -> Option<VarId> {
    call.get_parser_info(stack, DIR_VAR_PARSER_INFO)
        .and_then(|x| {
            if let Expr::Var(id) = x.expr {
                Some(id)
            } else {
                None
            }
        })
}

/// This helper function is used to find files during eval
///
/// First, the actual current working directory is selected as
///   a) the directory of a file currently being parsed
///   b) current working directory (PWD)
///
/// Then, if the file is not found in the actual cwd, NU_LIB_DIRS is checked.
/// If there is a relative path in NU_LIB_DIRS, it is assumed to be relative to the actual cwd
/// determined in the first step.
///
/// Always returns an absolute path
pub fn find_in_dirs_env(
    filename: &str,
    engine_state: &EngineState,
    stack: &Stack,
    dirs_var: Option<VarId>,
) -> Result<Option<PathBuf>, ShellError> {
    // Choose whether to use file-relative or PWD-relative path
    let cwd = if let Some(pwd) = stack.get_env_var(engine_state, "FILE_PWD") {
        match env_to_string("FILE_PWD", pwd, engine_state, stack) {
            Ok(cwd) => {
                if Path::new(&cwd).is_absolute() {
                    cwd
                } else {
                    return Err(ShellError::GenericError {
                            error: "Invalid current directory".into(),
                            msg: format!("The 'FILE_PWD' environment variable must be set to an absolute path. Found: '{cwd}'"),
                            span: Some(pwd.span()),
                            help: None,
                            inner: vec![]
                    });
                }
            }
            Err(e) => return Err(e),
        }
    } else {
        engine_state.cwd_as_string(Some(stack))?
    };

    let check_dir = |lib_dirs: Option<&Value>| -> Option<PathBuf> {
        if let Ok(p) = canonicalize_with(filename, &cwd) {
            return Some(p);
        }
        let path = Path::new(filename);
        if !path.is_relative() {
            return None;
        }

        lib_dirs?
            .as_list()
            .ok()?
            .iter()
            .map(|lib_dir| -> Option<PathBuf> {
                let dir = lib_dir.to_path().ok()?;
                let dir_abs = canonicalize_with(dir, &cwd).ok()?;
                canonicalize_with(filename, dir_abs).ok()
            })
            .find(Option::is_some)
            .flatten()
    };

    let lib_dirs = dirs_var.and_then(|var_id| engine_state.get_var(var_id).const_val.as_ref());
    // TODO: remove (see #8310)
    let lib_dirs_fallback = stack.get_env_var(engine_state, "NU_LIB_DIRS");

    Ok(check_dir(lib_dirs).or_else(|| check_dir(lib_dirs_fallback)))
}

fn get_converted_value(
    engine_state: &EngineState,
    stack: &Stack,
    name: &str,
    orig_val: &Value,
    direction: &str,
) -> Result<Value, ConversionError> {
    let conversion = stack
        .get_env_var(engine_state, ENV_CONVERSIONS)
        .ok_or(ConversionError::CellPathError)?
        .as_record()?
        .get(name)
        .ok_or(ConversionError::CellPathError)?
        .as_record()?
        .get(direction)
        .ok_or(ConversionError::CellPathError)?
        .as_closure()?;

    Ok(
        ClosureEvalOnce::new(engine_state, stack, conversion.clone())
            .debug(false)
            .run_with_value(orig_val.clone())?
            .into_value(orig_val.span())?,
    )
}

fn ensure_path(engine_state: &EngineState, stack: &mut Stack) -> Option<ShellError> {
    let mut error = None;

    // If PATH/Path is still a string, force-convert it to a list
    if let Some((preserve_case_name, value)) = stack.get_env_var_insensitive(engine_state, "Path") {
        let span = value.span();
        match value {
            Value::String { val, .. } => {
                // Force-split path into a list
                let paths = std::env::split_paths(val)
                    .map(|p| Value::string(p.to_string_lossy().to_string(), span))
                    .collect();

                stack.add_env_var(preserve_case_name.to_string(), Value::list(paths, span));
            }
            Value::List { vals, .. } => {
                // Must be a list of strings
                if !vals.iter().all(|v| matches!(v, Value::String { .. })) {
                    error = error.or_else(|| {
                        Some(ShellError::GenericError {
                            error: format!(
                                "Incorrect {preserve_case_name} environment variable value"
                            ),
                            msg: format!("{preserve_case_name} must be a list of strings"),
                            span: Some(span),
                            help: None,
                            inner: vec![],
                        })
                    });
                }
            }

            val => {
                // All other values are errors
                let span = val.span();

                error = error.or_else(|| {
                    Some(ShellError::GenericError {
                        error: format!("Incorrect {preserve_case_name} environment variable value"),
                        msg: format!("{preserve_case_name} must be a list of strings"),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    })
                });
            }
        }
    }

    error
}
