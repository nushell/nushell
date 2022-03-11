use std::collections::HashMap;
use std::path::{Path, PathBuf};

use nu_protocol::ast::PathMember;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{PipelineData, ShellError, Span, Value};

use crate::eval_block;

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
    GeneralError(ShellError),    // Other error not directly connected to running the conversion
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

    for (name, val) in &engine_state.env_vars {
        match get_converted_value(engine_state, stack, name, val, "from_string") {
            ConversionResult::Ok(v) => {
                let _ = new_scope.insert(name.to_string(), v);
            }
            ConversionResult::ConversionError(e) => error = error.or(Some(e)),
            ConversionResult::GeneralError(_) => continue,
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

    for (k, v) in new_scope {
        engine_state.env_vars.insert(k, v);
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
        ConversionResult::Ok(v) => Ok(v.as_string()?),
        ConversionResult::ConversionError(e) => Err(e),
        ConversionResult::GeneralError(e) => Err(e),
        ConversionResult::CellPathError => match value.as_string() {
            Ok(s) => Ok(s),
            Err(_) => {
                if env_name == ENV_PATH_NAME {
                    // Try to convert PATH/Path list to a string
                    match value {
                        Value::List { vals, .. } => {
                            let paths = vals
                                .iter()
                                .map(|v| v.as_string())
                                .collect::<Result<Vec<_>, _>>()?;

                            match std::env::join_paths(paths) {
                                Ok(p) => Ok(p.to_string_lossy().to_string()),
                                Err(_) => Err(ShellError::EnvVarNotAString(
                                    env_name.to_string(),
                                    value.span()?,
                                )),
                            }
                        }
                        _ => Err(ShellError::EnvVarNotAString(
                            env_name.to_string(),
                            value.span()?,
                        )),
                    }
                } else {
                    Err(ShellError::EnvVarNotAString(
                        env_name.to_string(),
                        value.span()?,
                    ))
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
            Err(ShellError::EnvVarNotAString(..)) => {} // ignore non-string values
            Err(e) => return Err(e),
        }
    }

    Ok(env_vars_str)
}

/// Shorthand for env_to_string() for PWD with custom error
pub fn current_dir_str(engine_state: &EngineState, stack: &Stack) -> Result<String, ShellError> {
    if let Some(pwd) = stack.get_env_var(engine_state, "PWD") {
        match env_to_string("PWD", &pwd, engine_state, stack) {
            Ok(cwd) => {
                if Path::new(&cwd).is_absolute() {
                    Ok(cwd)
                } else {
                    Err(ShellError::SpannedLabeledError(
                            "Invalid current directory".to_string(),
                            format!("The 'PWD' environment variable must be set to an absolute path. Found: '{}'", cwd),
                            pwd.span()?
                    ))
                }
            }
            Err(e) => Err(e),
        }
    } else {
        Err(ShellError::LabeledError(
                "Current directory not found".to_string(),
                "The environment variable 'PWD' was not found. It is required to define the current directory.".to_string(),
        ))
    }
}

/// Calls current_dir_str() and returns the current directory as a PathBuf
pub fn current_dir(engine_state: &EngineState, stack: &Stack) -> Result<PathBuf, ShellError> {
    current_dir_str(engine_state, stack).map(PathBuf::from)
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
                None => Err(ShellError::EnvVarNotFoundAtRuntime(
                    ENV_PATH_NAME_SECONDARY.to_string(),
                    span,
                )),
            }
            #[cfg(not(windows))]
            Err(ShellError::EnvVarNotFoundAtRuntime(
                ENV_PATH_NAME.to_string(),
                span,
            ))
        }
    }?;

    env_to_string(pathname, &pathval, engine_state, stack)
}

fn get_converted_value(
    engine_state: &EngineState,
    stack: &Stack,
    name: &str,
    orig_val: &Value,
    direction: &str,
) -> ConversionResult {
    if let Some(env_conversions) = stack.get_env_var(engine_state, ENV_CONVERSIONS) {
        let env_span = match env_conversions.span() {
            Ok(span) => span,
            Err(e) => {
                return ConversionResult::GeneralError(e);
            }
        };
        let val_span = match orig_val.span() {
            Ok(span) => span,
            Err(e) => {
                return ConversionResult::GeneralError(e);
            }
        };

        let path_members = &[
            PathMember::String {
                val: name.to_string(),
                span: env_span,
            },
            PathMember::String {
                val: direction.to_string(),
                span: env_span,
            },
        ];

        if let Ok(Value::Block {
            val: block_id,
            span: from_span,
            ..
        }) = env_conversions.follow_cell_path(path_members)
        {
            let block = engine_state.get_block(block_id);

            if let Some(var) = block.signature.get_positional(0) {
                let mut stack = stack.gather_captures(&block.captures);
                if let Some(var_id) = &var.var_id {
                    stack.add_var(*var_id, orig_val.clone());
                }

                let result = eval_block(
                    engine_state,
                    &mut stack,
                    block,
                    PipelineData::new(val_span),
                    true,
                    true,
                );

                match result {
                    Ok(data) => ConversionResult::Ok(data.into_value(val_span)),
                    Err(e) => ConversionResult::ConversionError(e),
                }
            } else {
                ConversionResult::ConversionError(ShellError::MissingParameter(
                    "block input".into(),
                    from_span,
                ))
            }
        } else {
            ConversionResult::CellPathError
        }
    } else {
        ConversionResult::CellPathError
    }
}

fn ensure_path(scope: &mut HashMap<String, Value>, env_path_name: &str) -> Option<ShellError> {
    let mut error = None;

    // If PATH/Path is still a string, force-convert it to a list
    match scope.get(env_path_name) {
        Some(Value::String { val, span }) => {
            // Force-split path into a list
            let span = *span;
            let paths = std::env::split_paths(val)
                .map(|p| Value::String {
                    val: p.to_string_lossy().to_string(),
                    span,
                })
                .collect();

            scope.insert(env_path_name.to_string(), Value::List { vals: paths, span });
        }
        Some(Value::List { vals, span }) => {
            // Must be a list of strings
            if !vals.iter().all(|v| matches!(v, Value::String { .. })) {
                error = error.or_else(|| {
                    Some(ShellError::SpannedLabeledError(
                        format!("Wrong {} environment variable value", env_path_name),
                        format!("{} must be a list of strings", env_path_name),
                        *span,
                    ))
                });
            }
        }
        Some(val) => {
            // All other values are errors
            let span = match val.span() {
                Ok(sp) => sp,
                Err(e) => {
                    error = error.or(Some(e));
                    Span::test_data() // FIXME: any better span to use here?
                }
            };

            error = error.or_else(|| {
                Some(ShellError::SpannedLabeledError(
                    format!("Wrong {} environment variable value", env_path_name),
                    format!("{} must be a list of strings", env_path_name),
                    span,
                ))
            });
        }
        None => { /* not preset, do nothing */ }
    }

    error
}
