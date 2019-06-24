crate mod hir;
crate mod parse2;
crate mod parse_command;
crate mod registry;

use crate::errors::ShellError;

crate use hir::baseline_parse_tokens::baseline_parse_tokens;
crate use parse2::call_node::CallNode;
crate use parse2::files::Files;
crate use parse2::flag::Flag;
crate use parse2::operator::Operator;
crate use parse2::parser::{nom_input, pipeline};
crate use parse2::pipeline::{Pipeline, PipelineElement};
crate use parse2::span::{Span, Spanned};
crate use parse2::text::Text;
crate use parse2::token_tree::TokenNode;
crate use parse2::tokens::{RawToken, Token};
crate use parse2::unit::Unit;
crate use parse_command::parse_command;
crate use registry::CommandRegistry;

pub fn parse(input: &str) -> Result<TokenNode, ShellError> {
    let _ = pretty_env_logger::try_init();

    match pipeline(nom_input(input)) {
        Ok((_rest, val)) => Ok(val),
        Err(err) => Err(ShellError::parse_error(err)),
    }
}
