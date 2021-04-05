use std::path::PathBuf;

use nu_errors::ShellError;

/// NuScript is either directly some nu code or
/// a file path to a nu-script file.
pub enum NuScript {
    Content(String),
    File(PathBuf),
}

impl NuScript {
    pub fn get_code(self) -> Result<String, ShellError> {
        match self {
            NuScript::Content(code) => Ok(code),
            NuScript::File(path) => std::fs::read_to_string(path).map_err(|e| {
                ShellError::untagged_runtime_error(format!("Reading of script failed with: {}", e))
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RunScriptOptions {
    pub with_cwd: Option<PathBuf>,
    pub with_stdin: bool,
    pub redirect_stdin: bool,
    pub exit_on_error: bool,
    pub cli_mode: bool,
    pub span_offset: usize,
    pub source_script: bool,
}

impl Default for RunScriptOptions {
    fn default() -> Self {
        Self {
            with_cwd: None,
            with_stdin: true,
            redirect_stdin: false,
            exit_on_error: true,
            cli_mode: false,
            span_offset: 0,
            source_script: false,
        }
    }
}

impl RunScriptOptions {
    /// The cwd the script shall run with
    /// No local configs aka nu-env files are loaded/unloaded by using a cwd other
    /// than the current one.
    pub fn with_cwd(mut self, path: PathBuf) -> Self {
        self.with_cwd = Some(path);
        self
    }

    pub fn with_stdin(mut self, stdin: bool) -> Self {
        self.with_stdin = stdin;
        self
    }

    pub fn redirect_stdin(mut self, redirect: bool) -> Self {
        self.redirect_stdin = redirect;
        self
    }

    pub fn exit_on_error(mut self, exit_on_error: bool) -> Self {
        self.exit_on_error = exit_on_error;
        self
    }

    pub fn cli_mode(mut self, cli_mode: bool) -> Self {
        self.cli_mode = cli_mode;
        self
    }

    pub fn span_offset(mut self, span_offset: usize) -> Self {
        self.span_offset = span_offset;
        self
    }

    pub fn source_script(mut self, source_file: bool) -> Self {
        self.source_script = source_file;
        self
    }
}
