<<<<<<< HEAD
#[macro_use]
mod macros;

mod call_info;
pub mod config_path;
pub mod hir;
mod maybe_owned;
mod registry;
mod return_value;
mod signature;
mod syntax_shape;
mod type_name;
mod type_shape;
pub mod value;

#[cfg(feature = "dataframe")]
pub mod dataframe;

pub use crate::call_info::{CallInfo, EvaluatedArgs};
pub use crate::config_path::ConfigPath;
pub use crate::maybe_owned::MaybeOwned;
pub use crate::registry::{SignatureRegistry, VariableRegistry};
pub use crate::return_value::{CommandAction, ReturnSuccess, ReturnValue};
pub use crate::signature::{NamedType, PositionalType, Signature};
pub use crate::syntax_shape::SyntaxShape;
pub use crate::type_name::{PrettyType, ShellTypeName, SpannedTypeName};
pub use crate::type_shape::{Row as RowType, Type};
pub use crate::value::column_path::{ColumnPath, PathMember, UnspannedPathMember};
pub use crate::value::dict::{Dictionary, TaggedDictBuilder};
pub use crate::value::did_you_mean::did_you_mean;
pub use crate::value::primitive::Primitive;
pub use crate::value::primitive::{format_date, format_duration, format_primitive};
pub use crate::value::range::{Range, RangeInclusion};
pub use crate::value::value_structure::{ValueResource, ValueStructure};
pub use crate::value::{merge_descriptors, UntaggedValue, Value};
=======
pub mod ast;
mod config;
pub mod engine;
mod example;
mod exportable;
mod id;
mod overlay;
mod pipeline_data;
mod shell_error;
mod signature;
mod span;
mod syntax_shape;
mod ty;
mod value;
pub use value::Value;

pub use config::*;
pub use engine::{
    CONFIG_VARIABLE_ID, ENV_VARIABLE_ID, IN_VARIABLE_ID, NU_VARIABLE_ID, SCOPE_VARIABLE_ID,
};
pub use example::*;
pub use exportable::*;
pub use id::*;
pub use overlay::*;
pub use pipeline_data::*;
pub use shell_error::*;
pub use signature::*;
pub use span::*;
pub use syntax_shape::*;
pub use ty::*;
pub use value::CustomValue;
pub use value::*;
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
