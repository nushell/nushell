#![cfg_attr(not(feature = "os"), allow(unused))]
#![doc = include_str!("../README.md")]
mod alias;
pub mod ast;
pub mod casing;
pub mod config;
pub mod debugger;
mod deprecation;
mod did_you_mean;
pub mod engine;
mod errors;
pub mod eval_base;
pub mod eval_const;
mod example;
mod id;
pub mod ir;
mod lev_distance;
mod module;
pub mod parser_path;
mod pipeline;
#[cfg(feature = "plugin")]
mod plugin;
#[cfg(feature = "os")]
pub mod process;
mod signature;
pub mod span;
mod syntax_shape;
mod ty;
mod value;

pub use alias::*;
pub use ast::unit::*;
pub use config::*;
pub use deprecation::*;
pub use did_you_mean::did_you_mean;
pub use engine::{ENV_VARIABLE_ID, IN_VARIABLE_ID, NU_VARIABLE_ID};
pub use errors::*;
pub use example::*;
pub use id::*;
pub use lev_distance::levenshtein_distance;
pub use module::*;
pub use pipeline::*;
#[cfg(feature = "plugin")]
pub use plugin::*;
pub use signature::*;
pub use span::*;
pub use syntax_shape::*;
pub use ty::*;
pub use value::*;

pub use nu_derive_value::*;
