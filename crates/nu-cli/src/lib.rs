#![doc = include_str!("../README.md")]
mod commands;
mod completions;
mod config_files;
mod eval_cmds;
mod eval_file;
mod menus;
mod nu_highlight;
mod print;
mod prompt;
mod prompt_update;
mod reedline_config;
mod repl;
mod syntax_highlight;
mod util;
mod validation;

pub use commands::add_cli_context;
pub use completions::{FileCompletion, NuCompleter, SemanticSuggestion, SuggestionKind};
pub use config_files::eval_config_contents;
pub use eval_cmds::{evaluate_commands, EvaluateCommandsOpts};
pub use eval_file::evaluate_file;
pub use menus::NuHelpCompleter;
pub use nu_highlight::NuHighlight;
pub use print::Print;
pub use prompt::NushellPrompt;
pub use repl::{do_auto_cd, evaluate_repl};
pub use syntax_highlight::NuHighlighter;
pub use util::{eval_source, gather_parent_env_vars};
pub use validation::NuValidator;

#[cfg(feature = "plugin")]
pub use config_files::add_plugin_file;
#[cfg(feature = "plugin")]
pub use config_files::migrate_old_plugin_file;
#[cfg(feature = "plugin")]
pub use config_files::read_plugin_file;
