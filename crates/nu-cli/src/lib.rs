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
//mod commands;
#[cfg(feature = "rustyline-support")]
mod completion;
mod env;
mod format;
//mod futures;
#[cfg(feature = "rustyline-support")]
mod git;
#[cfg(feature = "rustyline-support")]
mod keybinding;
mod line_editor;
//pub mod script;
mod shell;
pub mod types;
//pub mod utils;

//#[cfg(test)]
//mod examples;

#[cfg(feature = "rustyline-support")]
pub use crate::cli::cli;

pub use crate::cli::{parse_and_eval, register_plugins, run_script_file};

pub use nu_command::commands::default_context::create_default_context;
pub use crate::env::environment_syncer::EnvironmentSyncer;
pub use nu_data::config;
pub use nu_data::dict::TaggedListBuilder;
pub use nu_data::primitive;
pub use nu_data::value;
pub use nu_stream::{InputStream, InterruptibleStream, OutputStream};
pub use nu_value_ext::ValueExt;
pub use num_traits::cast::ToPrimitive;

// TODO: Temporary redirect
pub use nu_protocol::{did_you_mean, TaggedDictBuilder};
