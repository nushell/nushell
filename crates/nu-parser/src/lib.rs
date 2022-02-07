<<<<<<< HEAD
#[macro_use]
extern crate derive_new;

mod errors;
mod flag;
mod lex;
mod parse;
mod scope;
mod shapes;

pub use lex::lexer::{lex, parse_block, NewlineMode};
pub use lex::tokens::{LiteBlock, LiteCommand, LiteGroup, LitePipeline};
pub use parse::{classify_block, garbage, parse, parse_full_column_path, parse_math_expression};
pub use scope::ParserScope;
pub use shapes::shapes;
=======
mod errors;
mod flatten;
mod lex;
mod lite_parse;
mod parse_keywords;
mod parser;
mod type_check;

pub use errors::ParseError;
pub use flatten::{
    flatten_block, flatten_expression, flatten_pipeline, flatten_statement, FlatShape,
};
pub use lex::{lex, Token, TokenContents};
pub use lite_parse::{lite_parse, LiteBlock};

pub use parser::{find_captures_in_expr, parse, parse_block, trim_quotes, Import};

#[cfg(feature = "plugin")]
pub use parse_keywords::parse_register;
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
