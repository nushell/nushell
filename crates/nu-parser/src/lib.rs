mod deparse;
mod errors;
mod flatten;
mod known_external;
mod lex;
mod lite_parse;
mod parse_keywords;
mod parser;
mod type_check;

pub use deparse::{escape_for_script_arg, escape_quote_string};
pub use errors::ParseError;
pub use flatten::{flatten_block, flatten_expression, flatten_pipeline, FlatShape};
pub use known_external::KnownExternal;
pub use lex::{lex, Token, TokenContents};
pub use lite_parse::{lite_parse, LiteBlock};
pub use parse_keywords::*;

pub use parser::{
    is_math_expression_like, parse, parse_block, parse_duration_bytes, parse_expression,
    parse_external_call, trim_quotes, trim_quotes_str, unescape_unquote_string, Import,
};

#[cfg(feature = "plugin")]
pub use parse_keywords::parse_register;
