#![feature(crate_visibility_modifier)]
#![feature(in_band_lifetimes)]
#![feature(async_await)]
#![feature(try_trait)]
#![feature(bind_by_move_pattern_guards)]

mod cli;
mod commands;
mod context;
mod env;
mod errors;
mod evaluate;
mod format;
mod git;
mod object;
mod parser;
mod prelude;
mod shell;
mod stream;

pub use crate::commands::command::ReturnValue;
pub use crate::parser::Spanned;
pub use cli::cli;
pub use errors::ShellError;
pub use object::base::{Primitive, Value};
pub use parser::parse::text::Text;
