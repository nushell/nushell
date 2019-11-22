pub(crate) mod debug;
pub(crate) mod deserializer;
pub(crate) mod hir;
pub(crate) mod parse;
pub(crate) mod parse_command;
pub(crate) mod registry;

use crate::errors::ShellError;

pub(crate) use deserializer::ConfigDeserializer;
pub(crate) use hir::syntax_shape::flat_shape::FlatShape;
pub(crate) use hir::TokensIterator;
pub(crate) use parse::call_node::CallNode;
pub(crate) use parse::files::Files;
pub(crate) use parse::flag::{Flag, FlagKind};
pub(crate) use parse::operator::Operator;
pub(crate) use parse::parser::{nom_input, pipeline};
pub(crate) use parse::text::Text;
pub(crate) use parse::token_tree::{DelimitedNode, Delimiter, TokenNode};
pub(crate) use parse::tokens::{RawNumber, RawToken};
pub(crate) use parse::unit::Unit;
pub(crate) use registry::CommandRegistry;

pub fn parse(input: &str) -> Result<TokenNode, ShellError> {
    let _ = pretty_env_logger::try_init();

    match pipeline(nom_input(input)) {
        Ok((_rest, val)) => Ok(val),
        Err(err) => Err(ShellError::parse_error(err)),
    }
}
