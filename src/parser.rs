crate mod ast;
crate mod completer;
crate mod lexer;
crate mod parser;
crate mod registry;
crate mod span;

crate use ast::{ParsedCommand, Pipeline};
crate use registry::{CommandConfig, CommandRegistry};

use crate::errors::ShellError;
use lexer::Lexer;
use parser::PipelineParser;

pub fn parse(input: &str, _registry: &dyn CommandRegistry) -> Result<Pipeline, ShellError> {
    let parser = PipelineParser::new();
    let tokens = Lexer::new(input, false);

    match parser.parse(tokens) {
        Ok(val) => Ok(val),
        Err(err) => Err(ShellError::parse_error(err, input.to_string())),
    }
}
