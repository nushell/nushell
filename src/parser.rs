crate mod completer;
crate mod parser;
crate mod registry;
crate mod tokens;

crate use registry::{CommandConfig, CommandRegistry};
crate use tokens::{ParsedCommand, Pipeline};

use crate::errors::ShellError;
use parser::PipelineParser;

pub fn parse(input: &str, _registry: &dyn CommandRegistry) -> Result<Pipeline, ShellError> {
    let parser = PipelineParser::new();

    parser
        .parse(input)
        .map_err(|e| ShellError::string(format!("{:?}", e)))
}
