#![doc = include_str!("../README.md")]
pub mod commands;
pub mod fs;
pub mod locale_override;
pub mod macros;
pub mod playground;
use std::process::ExitStatus;

// Needs to be reexported for `nu!` macro
pub use nu_path;

#[derive(Debug)]
pub struct Outcome {
    pub out: String,
    pub err: String,
    pub status: ExitStatus,
}

#[cfg(windows)]
pub const NATIVE_PATH_ENV_VAR: &str = "Path";
#[cfg(not(windows))]
pub const NATIVE_PATH_ENV_VAR: &str = "PATH";

#[cfg(windows)]
pub const NATIVE_PATH_ENV_SEPARATOR: char = ';';
#[cfg(not(windows))]
pub const NATIVE_PATH_ENV_SEPARATOR: char = ':';

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

    if let Some(paths) = std::env::var_os(NATIVE_PATH_ENV_VAR) {
        original_paths = std::env::split_paths(&paths).collect::<Vec<_>>();
    }

    original_paths
}
