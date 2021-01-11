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

pub mod commands;
#[cfg(feature = "rustyline-support")]
mod completion;
mod futures;
pub mod maybe_print_errors;
pub mod script;
pub mod utils;

#[cfg(test)]
mod examples;

pub use crate::maybe_print_errors::maybe_print_errors;

pub use nu_data::config;
pub use nu_data::dict::TaggedListBuilder;
pub use nu_data::primitive;
pub use nu_data::value;
pub use nu_stream::{InputStream, InterruptibleStream, OutputStream};
pub use nu_value_ext::ValueExt;
pub use num_traits::cast::ToPrimitive;

// TODO: Temporary redirect
pub use nu_protocol::{did_you_mean, TaggedDictBuilder};
