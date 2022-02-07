<<<<<<< HEAD
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
=======
mod completions;
mod errors;
mod nu_highlight;
mod prompt;
mod syntax_highlight;
mod validation;

pub use completions::NuCompleter;
pub use errors::CliError;
pub use nu_highlight::NuHighlight;
pub use prompt::NushellPrompt;
pub use syntax_highlight::NuHighlighter;
pub use validation::NuValidator;
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
