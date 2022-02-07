<<<<<<< HEAD
mod call_info;
mod command_args;
mod config_holder;
pub mod documentation;
mod env;
pub mod evaluate;
pub mod evaluation_context;
mod example;
pub mod filesystem;
mod from_value;
mod maybe_text_codec;
pub mod plugin;
mod print;
pub mod script;
pub mod shell;
mod types;
mod whole_stream_command;

pub use crate::call_info::UnevaluatedCallInfo;
pub use crate::command_args::{CommandArgs, RunnableContext};
pub use crate::config_holder::ConfigHolder;
pub use crate::documentation::{generate_docs, get_brief_help, get_documentation, get_full_help};
pub use crate::env::host::FakeHost;
pub use crate::env::host::Host;
pub use crate::evaluate::block::run_block;
pub use crate::evaluate::envvar::EnvVar;
pub use crate::evaluate::scope::Scope;
pub use crate::evaluate::{evaluator, evaluator::evaluate_baseline_expr};
pub use crate::evaluation_context::EvaluationContext;
pub use crate::example::Example;
pub use crate::filesystem::dir_info::{DirBuilder, DirInfo, FileInfo};
pub use crate::filesystem::filesystem_shell::FilesystemShell;
pub use crate::from_value::FromValue;
pub use crate::maybe_text_codec::{BufCodecReader, MaybeTextCodec, StringOrBinary};
pub use crate::print::maybe_print_errors;
pub use crate::shell::painter::Painter;
pub use crate::shell::palette::{DefaultPalette, Palette};
pub use crate::shell::shell_manager::ShellManager;
pub use crate::shell::value_shell;
pub use crate::whole_stream_command::{whole_stream_command, Command, WholeStreamCommand};
=======
mod call_ext;
pub mod column;
pub mod documentation;
pub mod env;
mod eval;
mod glob_from;

pub use call_ext::CallExt;
pub use column::get_columns;
pub use documentation::{generate_docs, get_brief_help, get_documentation, get_full_help};
pub use env::*;
pub use eval::{
    eval_block, eval_block_with_redirect, eval_expression, eval_expression_with_input,
    eval_operator, eval_subexpression,
};
pub use glob_from::glob_from;
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
