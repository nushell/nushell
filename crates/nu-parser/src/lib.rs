#![doc = include_str!("../README.md")]
mod deparse;
mod exportable;
mod flatten;
mod known_external;
mod lex;
mod lite_parser;
mod parse_alias;
mod parse_bindings;
mod parse_calls;
mod parse_captures_compile;
mod parse_def;
mod parse_expressions;
mod parse_helpers;
mod parse_keywords;
mod parse_literals;
mod parse_module;
mod parse_patterns;
mod parse_pipelines;
mod parse_shape_specs;
mod parse_signatures;
mod parse_source;
mod parser;
mod type_check;

pub use deparse::escape_for_script_arg;
pub use flatten::{
    FlatShape, flatten_block, flatten_expression, flatten_pipeline, flatten_pipeline_element,
};
pub use known_external::KnownExternal;
pub use lex::{LexState, Token, TokenContents, lex, lex_n_tokens, lex_signature};
pub use lite_parser::{LiteBlock, LiteCommand, lite_parse};
pub use nu_protocol::parser_path::*;
pub use parse_keywords::*;

pub use parser::{
    DURATION_UNIT_GROUPS, is_math_expression_like, parse, parse_block, parse_expression,
    parse_external_call, parse_unit_value, trim_quotes, trim_quotes_str, unescape_unquote_string,
};
