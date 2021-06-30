pub mod app;
mod cli;
#[cfg(feature = "rustyline-support")]
mod keybinding;
mod line_editor;
#[cfg(feature = "rustyline-support")]
mod shell;

#[cfg(feature = "rustyline-support")]
pub use crate::cli::cli;

pub use crate::app::App;
pub use crate::cli::{parse_and_eval, register_plugins, run_script_file};

pub use nu_command::create_default_context;
