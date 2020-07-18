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
mod commands;
mod completion;
mod context;
pub mod data;
mod deserializer;
mod documentation;
mod env;
mod evaluate;
mod format;
mod futures;
mod git;
mod keybinding;
mod path;
mod shell;
mod stream;
pub mod utils;

#[cfg(test)]
mod examples;

pub use crate::cli::{
    cli, create_default_context, load_plugins, parse_and_eval, process_line,
    run_pipeline_standalone, run_vec_of_pipelines, LineResult,
};
pub use crate::commands::command::{
    whole_stream_command, CommandArgs, EvaluatedWholeStreamCommandArgs, Example, WholeStreamCommand,
};
pub use crate::commands::help::get_help;
pub use crate::context::{CommandRegistry, Context};
pub use crate::data::config;
pub use crate::data::dict::TaggedListBuilder;
pub use crate::data::primitive;
pub use crate::data::value;
pub use crate::env::environment_syncer::EnvironmentSyncer;
pub use crate::env::host::BasicHost;
pub use crate::prelude::ToOutputStream;
pub use crate::stream::{InputStream, InterruptibleStream, OutputStream};
pub use nu_value_ext::ValueExt;
pub use num_traits::cast::ToPrimitive;

// TODO: Temporary redirect
pub use nu_protocol::{did_you_mean, TaggedDictBuilder};
