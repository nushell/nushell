use crate::parser::parse2::{operator::*, span::*, tokens::*};
use derive_new::new;
use enum_utils::FromStr;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum TokenNode {
    Token(Token),
    Call(Spanned<CallNode>),
    Delimited(Spanned<DelimitedNode>),
    Pipeline(Spanned<Vec<TokenNode>>),
    Binary(Spanned<BinaryNode>),
    Path(Spanned<PathNode>),
}

impl TokenNode {
    pub fn span(&self) -> Span {
        match self {
            TokenNode::Token(t) => t.span,
            TokenNode::Call(s) => s.span,
            TokenNode::Delimited(s) => s.span,
            TokenNode::Pipeline(s) => s.span,
            TokenNode::Binary(s) => s.span,
            TokenNode::Path(s) => s.span,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, new)]
pub struct DelimitedNode {
    delimiter: Delimiter,
    children: Vec<TokenNode>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, new)]
pub struct CallNode {
    head: Box<TokenNode>,
    children: Vec<TokenNode>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, FromStr)]
pub enum Delimiter {
    Paren,
    Brace,
    Square,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, new)]
pub struct PathNode {
    head: Box<TokenNode>,
    tail: Vec<TokenNode>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, new)]
pub struct BinaryNode {
    left: Box<TokenNode>,
    op: Operator,
    right: Box<TokenNode>,
}
