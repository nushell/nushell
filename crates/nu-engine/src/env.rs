use crate::ClosureEvalOnce;
use nu_path::canonicalize_with;
use nu_protocol::{
    ast::{Call, Expr},
    engine::{EngineState, Stack, StateWorkingSet, PWD_ENV},
    Config, ShellError, Span, Value, VarId,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(windows)]
const ENV_PATH_NAME: &str = "Path";
#[cfg(windows)]
const ENV_PATH_NAME_SECONDARY: &str = "PATH";
#[cfg(not(windows))]
const ENV_PATH_NAME: &str = "PATH";

const ENV_CONVERSIONS: &str = "ENV_CONVERSIONS";

enum ConversionResult {
    Ok(Value),
    ConversionError(ShellError), // Failure during the conversion itself
    CellPathError, // Error looking up the ENV_VAR.to_/from_string fields in $env.ENV_CONVERSIONS
}

/// Translate environment variables from Strings to Values. Requires config to be already set up in
/// case the user defined custom env conversions in config.nu.
///
/// It returns Option instead of Result since we do want to translate all the values we can and
/// skip errors. This function is called in the main() so we want to keep running, we cannot just
/// exit.
pub fn convert_env_values(engine_state: &mut EngineState, stack: &Stack) -> Option<ShellError> {
    let mut error = None;

    let mut new_scope = HashMap::new();

    let env_vars = engine_state.render_env_vars();

    for (name, val) in env_vars {
        match get_converted_value(engine_state, stack, name, val, "from_string") {
            ConversionResult::Ok(v) => {
                let _ = new_scope.insert(name.to_string(), v);
            }
            ConversionResult::ConversionError(e) => error = error.or(Some(e)),
            ConversionResult::CellPathError => {
                let _ = new_scope.insert(name.to_string(), val.clone());
            }
        }
    }

    #[cfg(not(windows))]
    {
        error = error.or_else(|| ensure_path(&mut new_scope, ENV_PATH_NAME));
    }

    #[cfg(windows)]
    {
        let first_result = ensure_path(&mut new_scope, ENV_PATH_NAME);
        if first_result.is_some() {
            let second_result = ensure_path(&mut new_scope, ENV_PATH_NAME_SECONDARY);

            if second_result.is_some() {
                error = error.or(first_result);
            }
        }
    }

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

    error
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
        ConversionResult::Ok(v) => Ok(v.coerce_into_string()?),
        ConversionResult::ConversionError(e) => Err(e),
        ConversionResult::CellPathError => match value.coerce_string() {
            Ok(s) => Ok(s),
            Err(_) => {
                if env_name == ENV_PATH_NAME {
                    // Try to convert PATH/Path list to a string
                    match value {
                        Value::List { vals, .. } => {
                            let paths = vals
                                .iter()
                                .map(Value::coerce_str)
                                .collect::<Result<Vec<_>, _>>()?;

                            match std::env::join_paths(paths.iter().map(AsRef::as_ref)) {
                                Ok(p) => Ok(p.to_string_lossy().to_string()),
                                Err(_) => Err(ShellError::EnvVarNotAString {
                                    envvar_name: env_name.to_string(),
                                    span: value.span(),
                                }),
                            }
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

/// Shorthand for env_to_string() for PWD with custom error
pub fn current_dir_str(engine_state: &EngineState, stack: &Stack) -> Result<String, ShellError> {
    if let Some(pwd) = stack.get_env_var(engine_state, PWD_ENV) {
        // TODO: PWD should be string by default, we don't need to run ENV_CONVERSIONS on it
        match env_to_string(PWD_ENV, &pwd, engine_state, stack) {
            Ok(cwd) => {
                if Path::new(&cwd).is_absolute() {
                    Ok(cwd)
                } else {
                    Err(ShellError::GenericError {
                            error: "Invalid current directory".into(),
                            msg: format!("The 'PWD' environment variable must be set to an absolute path. Found: '{cwd}'"),
                            span: Some(pwd.span()),
                            help: None,
                            inner: vec![]
                    })
                }
            }
            Err(e) => Err(e),
        }
    } else {
        Err(ShellError::GenericError {
                error: "Current directory not found".into(),
                msg: "".into(),
                span: None,
                help: Some("The environment variable 'PWD' was not found. It is required to define the current directory.".into()),
                inner: vec![],
        })
    }
}

/// Simplified version of current_dir_str() for constant evaluation
pub fn current_dir_str_const(working_set: &StateWorkingSet) -> Result<String, ShellError> {
    if let Some(pwd) = working_set.get_env_var(PWD_ENV) {
        let span = pwd.span();
        match pwd {
            Value::String { val, .. } => {
                if Path::new(val).is_absolute() {
                    Ok(val.clone())
                } else {
                    Err(ShellError::GenericError {
                            error: "Invalid current directory".into(),
                            msg: format!("The 'PWD' environment variable must be set to an absolute path. Found: '{val}'"),
                            span: Some(span),
                            help: None,
                            inner: vec![]
                    })
                }
            }
            _ => Err(ShellError::GenericError {
                error: "PWD is not a string".into(),
                msg: "".into(),
                span: None,
                help: Some(
                    "Cusrrent working directory environment variable 'PWD' must be a string."
                        .into(),
                ),
                inner: vec![],
            }),
        }
    } else {
        Err(ShellError::GenericError{
                error: "Current directory not found".into(),
                msg: "".into(),
                span: None,
                help: Some("The environment variable 'PWD' was not found. It is required to define the current directory.".into()),
                inner: vec![],
        })
    }
}

/// Calls current_dir_str() and returns the current directory as a PathBuf
pub fn current_dir(engine_state: &EngineState, stack: &Stack) -> Result<PathBuf, ShellError> {
    current_dir_str(engine_state, stack).map(PathBuf::from)
}

/// Version of current_dir() for constant evaluation
pub fn current_dir_const(working_set: &StateWorkingSet) -> Result<PathBuf, ShellError> {
    current_dir_str_const(working_set).map(PathBuf::from)
}

/// Get the contents of path environment variable as a list of strings
///
/// On non-Windows: It will fetch PATH
/// On Windows: It will try to fetch Path first but if not present, try PATH
pub fn path_str(
    engine_state: &EngineState,
    stack: &Stack,
    span: Span,
) -> Result<String, ShellError> {
    let (pathname, pathval) = match stack.get_env_var(engine_state, ENV_PATH_NAME) {
        Some(v) => Ok((ENV_PATH_NAME, v)),
        None => {
            #[cfg(windows)]
            match stack.get_env_var(engine_state, ENV_PATH_NAME_SECONDARY) {
                Some(v) => Ok((ENV_PATH_NAME_SECONDARY, v)),
                None => Err(ShellError::EnvVarNotFoundAtRuntime {
                    envvar_name: ENV_PATH_NAME_SECONDARY.to_string(),
                    span,
                }),
            }
            #[cfg(not(windows))]
            Err(ShellError::EnvVarNotFoundAtRuntime {
                envvar_name: ENV_PATH_NAME.to_string(),
                span,
            })
        }
    }?;

    env_to_string(pathname, &pathval, engine_state, stack)
}

pub const DIR_VAR_PARSER_INFO: &str = "dirs_var";
pub fn get_dirs_var_from_call(call: &Call) -> Option<VarId> {
    call.get_parser_info(DIR_VAR_PARSER_INFO).and_then(|x| {
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
        match env_to_string("FILE_PWD", &pwd, engine_state, stack) {
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
        current_dir_str(engine_state, stack)?
    };

    let check_dir = |lib_dirs: Option<Value>| -> Option<PathBuf> {
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

    let lib_dirs = dirs_var.and_then(|var_id| engine_state.get_var(var_id).const_val.clone());
    // TODO: remove (see #8310)
    let lib_dirs_fallback = stack.get_env_var(engine_state, "NU_LIB_DIRS");

    Ok(check_dir(lib_dirs).or_else(|| check_dir(lib_dirs_fallback)))
}

/// Get config
///
/// This combines config stored in permanent state and any runtime updates to the environment. This
/// is the canonical way to fetch config at runtime when you have Stack available.
pub fn get_config(engine_state: &EngineState, stack: &Stack) -> Config {
    if let Some(mut config_record) = stack.get_env_var(engine_state, "config") {
        config_record.parse_as_config(engine_state.get_config()).0
    } else {
        engine_state.get_config().clone()
    }
}

fn get_converted_value(
    engine_state: &EngineState,
    stack: &Stack,
    name: &str,
    orig_val: &Value,
    direction: &str,
) -> ConversionResult {
    let conversions = stack.get_env_var(engine_state, ENV_CONVERSIONS);
    let conversion = conversions
        .as_ref()
        .and_then(|val| val.as_record().ok())
        .and_then(|record| record.get(name))
        .and_then(|val| val.as_record().ok())
        .and_then(|record| record.get(direction));

    if let Some(conversion) = conversion {
        match conversion.as_closure() {
            Ok(closure) => ClosureEvalOnce::new(engine_state, stack, closure.clone())
                .debug(false)
                .run_with_value(orig_val.clone())
                .map(|data| ConversionResult::Ok(data.into_value(orig_val.span())))
                .unwrap_or_else(ConversionResult::ConversionError),
            Err(e) => ConversionResult::ConversionError(e),
        }
    } else {
        ConversionResult::CellPathError
    }
}

fn ensure_path(scope: &mut HashMap<String, Value>, env_path_name: &str) -> Option<ShellError> {
    let mut error = None;

    // If PATH/Path is still a string, force-convert it to a list
    if let Some(value) = scope.get(env_path_name) {
        let span = value.span();
        match value {
            Value::String { val, .. } => {
                // Force-split path into a list
                let paths = std::env::split_paths(val)
                    .map(|p| Value::string(p.to_string_lossy().to_string(), span))
                    .collect();

                scope.insert(env_path_name.to_string(), Value::list(paths, span));
            }
            Value::List { vals, .. } => {
                // Must be a list of strings
                if !vals.iter().all(|v| matches!(v, Value::String { .. })) {
                    error = error.or_else(|| {
                        Some(ShellError::GenericError {
                            error: format!("Wrong {env_path_name} environment variable value"),
                            msg: format!("{env_path_name} must be a list of strings"),
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
                        error: format!("Wrong {env_path_name} environment variable value"),
                        msg: format!("{env_path_name} must be a list of strings"),
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
