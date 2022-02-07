<<<<<<< HEAD
#![recursion_limit = "2048"]

#[cfg(test)]
#[macro_use]
extern crate indexmap;

#[macro_use]
mod prelude;
mod classified;
pub mod commands;
mod default_context;
pub mod utils;

#[cfg(test)]
mod examples;

pub use crate::default_context::create_default_context;
pub use nu_data::config;
pub use nu_data::dict::TaggedListBuilder;
pub use nu_data::primitive;
pub use nu_data::value;
pub use nu_stream::{ActionStream, InputStream, InterruptibleStream};
pub use nu_value_ext::ValueExt;
pub use num_traits::cast::ToPrimitive;

// TODO: Temporary redirect
pub use nu_protocol::{did_you_mean, TaggedDictBuilder};
=======
mod conversions;
mod core_commands;
mod date;
mod default_context;
mod env;
mod example_test;
mod experimental;
mod filesystem;
mod filters;
mod formats;
mod generators;
mod hash;
mod math;
mod network;
mod path;
mod platform;
mod random;
mod shells;
mod strings;
mod system;
mod viewers;

pub use conversions::*;
pub use core_commands::*;
pub use date::*;
pub use default_context::*;
pub use env::*;
#[cfg(test)]
pub use example_test::test_examples;
pub use experimental::*;
pub use filesystem::*;
pub use filters::*;
pub use formats::*;
pub use generators::*;
pub use hash::*;
pub use math::*;
pub use network::*;
pub use path::*;
pub use platform::*;
pub use random::*;
pub use shells::*;
pub use strings::*;
pub use system::*;
pub use viewers::*;

#[cfg(feature = "dataframe")]
mod dataframe;

#[cfg(feature = "dataframe")]
pub use dataframe::*;
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
