use crate::parser::parse2::{span::*, tokens::*};
use derive_new::new;
use enum_utils::FromStr;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum TokenNode {
    Token(Token),
    Delimited(Spanned<DelimitedNode>),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, new)]
pub struct DelimitedNode {
    delimiter: Delimiter,
    children: Vec<TokenNode>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, FromStr)]
pub enum Delimiter {
    Paren,
    Brace,
    Square,
}
