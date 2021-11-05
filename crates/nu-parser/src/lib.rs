mod errors;
mod flatten;
mod lex;
mod lite_parse;
mod parse_keywords;
mod parser;
mod type_check;

pub use errors::ParseError;
pub use flatten::{flatten_block, FlatShape};
pub use lex::{lex, Token, TokenContents};
pub use lite_parse::{lite_parse, LiteBlock};
pub use parse_keywords::{
    parse_alias, parse_def, parse_def_predecl, parse_let, parse_module, parse_use,
};
pub use parser::{find_captures_in_expr, parse, Import, VarDecl};

#[cfg(feature = "plugin")]
pub use parse_keywords::parse_plugin;
