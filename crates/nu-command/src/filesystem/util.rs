use dialoguer::Input;
use nu_engine::{command_prelude::*, get_eval_expression};
use nu_protocol::{FromValue, NuGlob};
use std::{
    error::Error,
    path::{Path, PathBuf},
};

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Resource {
    pub at: usize,
    pub location: PathBuf,
}

impl Resource {}

pub fn try_interaction(
    interactive: bool,
    prompt: String,
) -> (Result<Option<bool>, Box<dyn Error>>, bool) {
    let interaction = if interactive {
        match get_interactive_confirmation(prompt) {
            Ok(i) => Ok(Some(i)),
            Err(e) => Err(e),
        }
    } else {
        Ok(None)
    };

    let confirmed = match interaction {
        Ok(maybe_input) => maybe_input.unwrap_or(false),
        Err(_) => false,
    };

    (interaction, confirmed)
}

fn get_interactive_confirmation(prompt: String) -> Result<bool, Box<dyn Error>> {
    let input = Input::new()
        .with_prompt(prompt)
        .validate_with(|c_input: &String| -> Result<(), String> {
            if c_input.len() == 1
                && (c_input == "y" || c_input == "Y" || c_input == "n" || c_input == "N")
            {
                Ok(())
            } else if c_input.len() > 1 {
                Err("Enter only one letter (Y/N)".to_string())
            } else {
                Err("Input not valid".to_string())
            }
        })
        .default("Y/N".into())
        .interact_text()?;

    if input == "y" || input == "Y" {
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Return `Some(true)` if the last change time of the `src` old than the `dst`,
/// otherwisie return `Some(false)`. Return `None` if the `src` or `dst` doesn't exist.
#[allow(dead_code)]
pub fn is_older(src: &Path, dst: &Path) -> Option<bool> {
    if !dst.exists() || !src.exists() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let src_ctime = std::fs::metadata(src)
            .map(|m| m.ctime())
            .unwrap_or(i64::MIN);
        let dst_ctime = std::fs::metadata(dst)
            .map(|m| m.ctime())
            .unwrap_or(i64::MAX);
        Some(src_ctime <= dst_ctime)
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        let src_ctime = std::fs::metadata(src)
            .map(|m| m.last_write_time())
            .unwrap_or(u64::MIN);
        let dst_ctime = std::fs::metadata(dst)
            .map(|m| m.last_write_time())
            .unwrap_or(u64::MAX);
        Some(src_ctime <= dst_ctime)
    }
}

/// Get rest arguments from given `call`, starts with `starting_pos`.
///
/// It's similar to `call.rest`, except that it always returns NuGlob.
pub fn get_rest_for_glob_pattern(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    starting_pos: usize,
) -> Result<Vec<Spanned<NuGlob>>, ShellError> {
    let eval_expression = get_eval_expression(engine_state);

    call.rest_iter_flattened(engine_state, stack, eval_expression, starting_pos)?
        .into_iter()
        // This used to be much more complex, but I think `FromValue` should be able to handle the
        // nuance here.
        .map(FromValue::from_value)
        .collect()
}
