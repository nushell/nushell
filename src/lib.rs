#![recursion_limit = "512"]

#[macro_use]
mod prelude;

mod cli;
mod commands;
mod context;
mod data;
mod env;
mod errors;
mod evaluate;
mod format;
mod fuzzysearch;
mod git;
mod parser;
mod plugin;
mod shell;
mod stream;
mod traits;
mod utils;

pub use crate::commands::command::{CallInfo, ReturnSuccess, ReturnValue};
pub use crate::context::{AnchorLocation, SourceMap};
pub use crate::env::host::BasicHost;
pub use crate::parser::hir::SyntaxShape;
pub use crate::parser::parse::token_tree_builder::TokenTreeBuilder;
pub use crate::plugin::{serve_plugin, Plugin};
pub use crate::utils::{AbsoluteFile, AbsolutePath, RelativePath};
pub use cli::cli;
pub use data::base::{Primitive, Value};
pub use data::config::{config_path, APP_INFO};
pub use data::dict::{Dictionary, TaggedDictBuilder};
pub use data::meta::{Span, Tag, Tagged, TaggedItem};
pub use errors::{CoerceInto, ShellError};
pub use num_traits::cast::ToPrimitive;
pub use parser::parse::text::Text;
pub use parser::registry::{EvaluatedArgs, NamedType, PositionalType, Signature};
