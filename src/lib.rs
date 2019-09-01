#![feature(crate_visibility_modifier)]
#![feature(generators)]
#![feature(specialization)]
#![feature(proc_macro_hygiene)]

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
mod traits;
mod utils;

pub use crate::commands::command::{CallInfo, ReturnSuccess, ReturnValue};
pub use crate::context::{SourceMap, SpanSource};
pub use crate::env::host::BasicHost;
pub use crate::parser::hir::SyntaxType;
pub use crate::plugin::{serve_plugin, Plugin};
pub use crate::utils::{AbsoluteFile, AbsolutePath, RelativePath};
pub use cli::cli;
pub use errors::{CoerceInto, ShellError};
pub use num_traits::cast::ToPrimitive;
pub use object::base::{Primitive, Value};
pub use object::dict::{Dictionary, TaggedDictBuilder};
pub use object::meta::{Span, Tag, Tagged, TaggedItem};
pub use parser::parse::text::Text;
pub use parser::registry::{EvaluatedArgs, NamedType, PositionalType, Signature};
