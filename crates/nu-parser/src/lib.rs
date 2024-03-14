mod deparse;
mod exportable;
mod flatten;
mod known_external;
mod lex;
mod lite_parser;
mod parse_keywords;
mod parse_patterns;
mod parse_shape_specs;
mod parser;
mod parser_path;
mod type_check;

pub use deparse::{escape_for_script_arg, escape_quote_string};
pub use flatten::{
    flatten_block, flatten_expression, flatten_pipeline, flatten_pipeline_element, FlatShape,
};
pub use known_external::KnownExternal;
pub use lex::{lex, lex_signature, Token, TokenContents};
pub use lite_parser::{lite_parse, LiteBlock, LiteCommand};
pub use parse_keywords::*;
pub use parser_path::*;

pub use parser::{
    is_math_expression_like, parse, parse_block, parse_expression, parse_external_call,
    parse_unit_value, trim_quotes, trim_quotes_str, unescape_unquote_string, DURATION_UNIT_GROUPS,
};

#[cfg(feature = "plugin")]
pub use parse_keywords::parse_register;
