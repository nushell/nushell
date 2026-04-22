//! Deprecated testing utilities.
//!
//! The utilities in this module are considered deprecated and new tests should no longer use them.
//! However they are not yet marked as `#[deprecated]` to avoid a massive amount of warnings during
//! compilation.

use std::process::ExitStatus;

pub mod commands;
pub mod locale_override;
pub mod macros;

#[derive(Debug)]
pub struct Outcome {
    pub out: String,
    pub err: String,
    pub status: ExitStatus,
}

impl Outcome {
    pub fn new(out: String, err: String, status: ExitStatus) -> Outcome {
        Outcome { out, err, status }
    }
}

pub fn nu_repl_code(source_lines: &[&str]) -> String {
    let mut out = String::from("nu --testbin=nu_repl ...[ ");

    for line in source_lines.iter() {
        out.push('`');
        out.push_str(line);
        out.push('`');
        out.push(' ');
    }

    out.push(']');

    out
}

pub fn shell_os_paths() -> Vec<std::path::PathBuf> {
    let mut original_paths = vec![];

    if let Some(paths) = std::env::var_os(nu_utils::consts::NATIVE_PATH_ENV_VAR) {
        original_paths = std::env::split_paths(&paths).collect::<Vec<_>>();
    }

    original_paths
}
