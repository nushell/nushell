
pub mod commands;
pub mod assertions;
pub mod fs;
pub mod harness;
pub mod locale_override;
pub mod macros;
pub mod net;
pub mod playground;

pub mod tester;
use nu_utils::container::Container;
pub use tester::{Result, ShellErrorExt, TestError as Error, TestResultExt, test};

pub mod prelude {
    #[doc(no_inline)]
    pub use super::{
        Outcome, assertions::*, nu,
        playground::Playground,
        tester::{Result, ShellErrorExt, TestError as Error, TestResultExt, test},
    };

    #[doc(no_inline)]
    pub use nu_protocol::{CompileError, FromValue, IntoValue, ParseError, ShellError, Value};
}

// Expose macros to be used for the test harness.
pub use harness::macros::*;

// Needs to be reexported for `nu!` macro
pub use nu_path;

// Export json macro to allow writing json values easily.
pub use serde_json::json;

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
