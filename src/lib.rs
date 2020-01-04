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
mod git;
mod shell;
mod stream;
mod utils;

pub use crate::cli::cli;
pub use crate::data::config::{config_path, APP_INFO};
pub use crate::data::dict::TaggedListBuilder;
pub use crate::data::primitive;
pub use crate::data::value;
pub use crate::env::host::BasicHost;
pub use nu_parser::TokenTreeBuilder;
pub use nu_value_ext::ValueExt;
pub use num_traits::cast::ToPrimitive;

// TODO: Temporary redirect
pub use nu_protocol::{did_you_mean, TaggedDictBuilder};
//pub use nu_plugin::{serve_plugin, Plugin};
