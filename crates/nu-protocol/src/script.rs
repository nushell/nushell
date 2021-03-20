use std::path::PathBuf;

extern crate derive_builder;

#[derive(Default, Debug, Clone, Builder)]
#[builder(setter(into))]
pub struct RunScriptOptions {
    pub with_cwd: Option<PathBuf>,
    pub with_stdin: bool,
    pub redirect_stdin: bool,
    pub exit_on_error: bool,
    pub cli_mode: bool,
    pub span_offset: usize,
}

impl RunScriptOptions {
    pub fn new() -> Self {
        Self {
            with_cwd: None,
            with_stdin: true,
            redirect_stdin: false,
            exit_on_error: true,
            cli_mode: false,
            span_offset: 0,
        }
    }
}
