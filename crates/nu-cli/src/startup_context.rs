//! Context for files loaded during Nushell startup (env/config/login/autoload).
//!
//! Startup load errors use the normal miette diagnostics (path/labels from spans).
//! No extra preface or continue banner — those duplicated what miette already shows.

use std::path::PathBuf;

use nu_protocol::{
    CompileError, ParseError, ShellError, Span,
    engine::{EngineState, Stack, StateWorkingSet},
    report_error::report_compile_error,
    report_parse_error, report_shell_error,
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

/// Identifies a startup load (path/role). Kept for call sites that track which
/// file is being evaluated; diagnostics themselves come from miette spans.
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

/// Report a parse error from a startup-evaluated source.
pub fn report_startup_parse_error(
    stack: Option<&Stack>,
    working_set: &StateWorkingSet,
    error: &ParseError,
    _startup: Option<&StartupLoadContext>,
) {
    report_parse_error(stack, working_set, error);
}

/// Report a compile error from a startup-evaluated source.
pub fn report_startup_compile_error(
    stack: Option<&Stack>,
    working_set: &StateWorkingSet,
    error: &CompileError,
    _startup: Option<&StartupLoadContext>,
) {
    report_compile_error(stack, working_set, error);
}

/// Report a shell error from a startup-evaluated source.
pub fn report_startup_shell_error(
    stack: Option<&Stack>,
    engine_state: &EngineState,
    error: &ShellError,
    _startup: Option<&StartupLoadContext>,
) {
    report_shell_error(stack, engine_state, error);
}

/// Report a missing/unreadable startup path without blaming Host Environment Variables.
pub fn report_startup_file_not_found(
    engine_state: &EngineState,
    path_display: &str,
    cli_span: Option<Span>,
    _startup: Option<&StartupLoadContext>,
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
            let msg = format!(
                "Error: File not found: {path_display}\n  help: Check the path passed to --config / --env-config, or create the file under your config directory."
            );
            if writeln_stderr(&msg).is_err() {
                let _ = writeln_stdout(&msg);
            }
        }
    }
}
