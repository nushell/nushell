use std::collections::HashMap;
use std::path::{Path, PathBuf};

use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{Config, PipelineData, ShellError, Value};

use crate::eval_block;

#[cfg(windows)]
const ENV_SEP: &str = ";";
#[cfg(not(windows))]
const ENV_SEP: &str = ":";

/// Translate environment variables from Strings to Values. Requires config to be already set up in
/// case the user defined custom env conversions in config.nu.
///
/// It returns Option instead of Result since we do want to translate all the values we can and
/// skip errors. This function is called in the main() so we want to keep running, we cannot just
/// exit.
pub fn convert_env_values(
    engine_state: &mut EngineState,
    stack: &Stack,
    config: &Config,
) -> Option<ShellError> {
    let mut error = None;

    let mut new_scope = HashMap::new();

    for (name, val) in &engine_state.env_vars {
        if let Some(env_conv) = config.env_conversions.get(name) {
            if let Some((block_id, from_span)) = env_conv.from_string {
                let val_span = match val.span() {
                    Ok(sp) => sp,
                    Err(e) => {
                        error = error.or(Some(e));
                        continue;
                    }
                };

                let block = engine_state.get_block(block_id);

                if let Some(var) = block.signature.get_positional(0) {
                    let mut stack = stack.gather_captures(&block.captures);
                    if let Some(var_id) = &var.var_id {
                        stack.add_var(*var_id, val.clone());
                    }

                    let result =
                        eval_block(engine_state, &mut stack, block, PipelineData::new(val_span));

                    match result {
                        Ok(data) => {
                            let val = data.into_value(val_span);
                            new_scope.insert(name.to_string(), val);
                        }
                        Err(e) => error = error.or(Some(e)),
                    }
                } else {
                    error = error.or_else(|| {
                        Some(ShellError::MissingParameter(
                            "block input".into(),
                            from_span,
                        ))
                    });
                }
            } else {
                new_scope.insert(name.to_string(), val.clone());
            }
        } else {
            new_scope.insert(name.to_string(), val.clone());
        }
    }

    for (k, v) in new_scope {
        engine_state.env_vars.insert(k, v);
    }

    error
}

/// Translate one environment variable from Value to String
pub fn env_to_string(
    env_name: &str,
    value: Value,
    engine_state: &EngineState,
    stack: &Stack,
    config: &Config,
) -> Result<String, ShellError> {
    if let Some(env_conv) = config.env_conversions.get(env_name) {
        if let Some((block_id, to_span)) = env_conv.to_string {
            let block = engine_state.get_block(block_id);

            if let Some(var) = block.signature.get_positional(0) {
                let val_span = value.span()?;
                let mut stack = stack.gather_captures(&block.captures);

                if let Some(var_id) = &var.var_id {
                    stack.add_var(*var_id, value);
                }

                Ok(
                    // This one is OK to fail: We want to know if custom conversion is working
                    eval_block(engine_state, &mut stack, block, PipelineData::new(val_span))?
                        .into_value(val_span)
                        .as_string()?,
                )
            } else {
                Err(ShellError::MissingParameter("block input".into(), to_span))
            }
        } else {
            // Do not fail here. Must succeed, otherwise setting a non-string env var would constantly
            // throw errors when running externals etc.
            Ok(value.into_string(ENV_SEP, config))
        }
    } else {
        // Do not fail here. Must succeed, otherwise setting a non-string env var would constantly
        // throw errors when running externals etc.
        Ok(value.into_string(ENV_SEP, config))
    }
}

/// Translate all environment variables from Values to Strings
pub fn env_to_strings(
    engine_state: &EngineState,
    stack: &Stack,
    config: &Config,
) -> Result<HashMap<String, String>, ShellError> {
    let env_vars = stack.get_env_vars(engine_state);
    let mut env_vars_str = HashMap::new();
    for (env_name, val) in env_vars {
        let val_str = env_to_string(&env_name, val, engine_state, stack, config)?;
        env_vars_str.insert(env_name, val_str);
    }

    Ok(env_vars_str)
}

/// Shorthand for env_to_string() for PWD with custom error
pub fn current_dir_str(engine_state: &EngineState, stack: &Stack) -> Result<String, ShellError> {
    let config = stack.get_config()?;
    if let Some(pwd) = stack.get_env_var(engine_state, "PWD") {
        match env_to_string("PWD", pwd, engine_state, stack, &config) {
            Ok(cwd) => {
                if Path::new(&cwd).is_absolute() {
                    Ok(cwd)
                } else {
                    println!("cwd is: {}", cwd);
                    Err(ShellError::LabeledError(
                            "Invalid current directory".to_string(),
                            format!("The 'PWD' environment variable must be set to an absolute path. Found: '{}'", cwd)
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
