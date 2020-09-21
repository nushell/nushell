#![recursion_limit = "2048"]

#[cfg(test)]
#[macro_use]
extern crate indexmap;

#[macro_use]
mod prelude;

#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

mod cli;
mod command_registry;
mod commands;
#[cfg(feature = "rustyline-support")]
mod completion;
mod deserializer;
mod documentation;
mod env;
mod evaluate;
mod evaluation_context;
mod format;
mod futures;
#[cfg(feature = "rustyline-support")]
mod git;
#[cfg(feature = "rustyline-support")]
mod keybinding;
mod path;
mod plugin;
mod shell;
mod stream;
pub mod utils;

#[cfg(test)]
mod examples;

#[cfg(feature = "rustyline-support")]
pub use crate::cli::cli;

pub use crate::cli::{
    create_default_context, parse_and_eval, process_line, register_plugins,
    run_pipeline_standalone, run_vec_of_pipelines, LineResult,
};
pub use crate::command_registry::CommandRegistry;
pub use crate::commands::command::{
    whole_stream_command, CommandArgs, EvaluatedWholeStreamCommandArgs, Example, WholeStreamCommand,
};
pub use crate::commands::help::get_help;
pub use crate::env::environment_syncer::EnvironmentSyncer;
pub use crate::env::host::BasicHost;
pub use crate::evaluation_context::EvaluationContext;
pub use crate::prelude::ToOutputStream;
pub use crate::stream::{InputStream, InterruptibleStream, OutputStream};
pub use nu_data::config;
pub use nu_data::dict::TaggedListBuilder;
pub use nu_data::primitive;
pub use nu_data::value;
pub use nu_value_ext::ValueExt;
pub use num_traits::cast::ToPrimitive;

// TODO: Temporary redirect
pub use nu_protocol::{did_you_mean, TaggedDictBuilder};
