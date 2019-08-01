crate mod hir;
crate mod parse;
crate mod parse_command;
crate mod registry;

use crate::errors::ShellError;

crate use hir::baseline_parse_tokens::baseline_parse_tokens;
crate use parse::call_node::CallNode;
crate use parse::files::Files;
crate use parse::flag::Flag;
crate use parse::operator::Operator;
crate use parse::parser::{nom_input, pipeline};
crate use parse::pipeline::{Pipeline, PipelineElement};
crate use parse::text::Text;
crate use parse::token_tree::{DelimitedNode, Delimiter, PathNode, TokenNode};
crate use parse::tokens::{RawToken, Token};
crate use parse::unit::Unit;
crate use parse_command::parse_command;
crate use registry::CommandRegistry;

pub fn parse(input: &str) -> Result<TokenNode, ShellError> {
    let _ = pretty_env_logger::try_init();

    match pipeline(nom_input(input)) {
        Ok((_rest, val)) => Ok(val),
        Err(err) => Err(ShellError::parse_error(err)),
    }
}
