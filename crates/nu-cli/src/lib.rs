#![recursion_limit = "2048"]

#[cfg(test)]
#[macro_use]
extern crate indexmap;

#[macro_use]
mod prelude;

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
mod shell;
mod stream;
mod utils;

pub use crate::cli::{cli, create_default_context, load_plugins, run_pipeline_standalone};
pub use crate::data::dict::TaggedListBuilder;
pub use crate::data::primitive;
pub use crate::data::value;
pub use crate::env::environment_syncer::EnvironmentSyncer;
pub use crate::env::host::BasicHost;
pub use nu_value_ext::ValueExt;
pub use num_traits::cast::ToPrimitive;

// TODO: Temporary redirect
pub use nu_protocol::{did_you_mean, TaggedDictBuilder};
