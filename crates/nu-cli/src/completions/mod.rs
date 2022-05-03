mod base;
mod command_completions;
mod completer;
mod completion_options;
mod custom_completions;
mod directory_completions;
mod dotnu_completions;
mod file_completions;
mod flag_completions;
mod variable_completions;

pub use base::Completer;
pub use command_completions::CommandCompletion;
pub use completer::NuCompleter;
pub use completion_options::{CompletionOptions, MatchAlgorithm, SortBy};
pub use custom_completions::CustomCompletion;
pub use directory_completions::DirectoryCompletion;
pub use dotnu_completions::DotNuCompletion;
pub use file_completions::{
    file_path_completion, matches, partial_from, prepend_base_dir, FileCompletion,
};
pub use flag_completions::FlagCompletion;
pub use variable_completions::VariableCompletion;
