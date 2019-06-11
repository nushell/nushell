use crate::parser::parse2::{span::*, tokens::*};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum TokenNode {
    Token(Token),
}
