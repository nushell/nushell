#![feature(crate_visibility_modifier)]
#![feature(in_band_lifetimes)]
#![feature(async_await)]
#![feature(try_trait)]
#![feature(bind_by_move_pattern_guards)]
#![feature(box_syntax)]
#![feature(type_ascription)]
#![feature(option_flattening)]

#[macro_use]
mod prelude;

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
mod plugin;
mod shell;
mod stream;

pub use crate::commands::command::{CallInfo, ReturnSuccess, ReturnValue};
pub use crate::context::{SourceMap, SpanSource};
pub use crate::env::host::BasicHost;
pub use crate::object::base::OF64;
pub use crate::plugin::{serve_plugin, Plugin};
pub use cli::cli;
pub use errors::ShellError;
pub use object::base::{Primitive, Value};
pub use object::dict::{Dictionary, TaggedDictBuilder};
pub use object::meta::{Span, Tag, Tagged, TaggedItem};
pub use parser::parse::text::Text;
pub use parser::registry::{Args, CommandConfig, NamedType, PositionalType};
