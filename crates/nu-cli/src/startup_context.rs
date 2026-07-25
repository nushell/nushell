//! Context for files loaded during Nushell startup (env/config/login/autoload).
//!
//! # Error reporting design
//!
//! Parse/compile/shell diagnostics use the normal miette reporters. Path and
//! labels come from the source file name passed into `parse` / spans on the
//! error — not from a custom preface or continue banner (those duplicated what
//! miette already shows).
//!
//! [`StartupLoadContext`] identifies *which* startup file is being loaded so
//! call sites can attach path/role to path-level failures (read errors, missing
//! override files) where there is no useful parse span.

use std::path::PathBuf;

use nu_protocol::{
    ParseError, Span,
    engine::{EngineState, StateWorkingSet},
    report_parse_error,
};

/// Which kind of startup file is being loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupFileKind {
    Env,
    Config,
    Login,
    Autoload,
    DefaultEnv,
    DefaultConfig,
}

impl StartupFileKind {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Env => "env.nu",
            Self::Config => "config.nu",
            Self::Login => "login.nu",
            Self::Autoload => "autoload",
            Self::DefaultEnv => "default_env.nu",
            Self::DefaultConfig => "default_config.nu",
        }
    }
}

/// Identifies a startup load (path and role).
///
/// Used when reporting path-level failures (missing/unreadable files).
/// Parse/compile/shell errors go through the standard reporters; their location
/// comes from miette spans and the evaluated source name.
#[derive(Debug, Clone)]
pub struct StartupLoadContext {
    pub kind: StartupFileKind,
    pub path: PathBuf,
}

impl StartupLoadContext {
    pub fn new(kind: StartupFileKind, path: impl Into<PathBuf>) -> Self {
        Self {
            kind,
            path: path.into(),
        }
    }
}

fn writeln_stderr(msg: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut err = std::io::stderr().lock();
    writeln!(err, "{msg}")
}

fn writeln_stdout(msg: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    writeln!(out, "{msg}")
}

/// Report a missing/unreadable startup path without blaming Host Environment Variables.
pub fn report_startup_file_not_found(
    engine_state: &EngineState,
    path_display: &str,
    cli_span: Option<Span>,
    startup: Option<&StartupLoadContext>,
) {
    match cli_span {
        Some(span) if span != Span::unknown() => {
            let working_set = StateWorkingSet::new(engine_state);
            report_parse_error(
                None,
                &working_set,
                &ParseError::FileNotFound(path_display.to_string(), span),
            );
        }
        _ => {
            // No real CLI span — avoid Span::unknown() (Host Environment Variables) and
            // new_internal (Rust source location). Plain message is clearest here.
            let role = startup
                .map(|s| s.kind.display_name())
                .unwrap_or("startup file");
            let msg = format!(
                "Error: File not found: {path_display} ({role})\n  help: Check the path passed to --config / --env-config, or create the file under your config directory."
            );
            if writeln_stderr(&msg).is_err() {
                let _ = writeln_stdout(&msg);
            }
        }
    }
}
