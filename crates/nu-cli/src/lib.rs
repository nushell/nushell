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
mod context;
mod data;
mod deserializer;
mod env;
mod evaluate;
mod format;
mod futures;
mod git;
mod path;
mod shell;
mod stream;
pub mod utils;

#[cfg(test)]
mod examples;

pub use crate::cli::{
    cli, create_default_context, load_plugins, run_pipeline_standalone, run_vec_of_pipelines,
};
pub use crate::commands::command::{
    whole_stream_command, CommandArgs, EvaluatedWholeStreamCommandArgs, WholeStreamCommand,
};
pub use crate::commands::help::get_help;
pub use crate::context::CommandRegistry;
pub use crate::data::dict::TaggedListBuilder;
pub use crate::data::primitive;
pub use crate::data::value;
pub use crate::env::environment_syncer::EnvironmentSyncer;
pub use crate::env::host::BasicHost;
pub use crate::stream::OutputStream;
pub use nu_value_ext::ValueExt;
pub use num_traits::cast::ToPrimitive;

// TODO: Temporary redirect
pub use nu_protocol::{did_you_mean, TaggedDictBuilder};
