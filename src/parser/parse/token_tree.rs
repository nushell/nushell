use crate::errors::ShellError;
use crate::parser::parse::{call_node::*, flag::*, operator::*, pipeline::*, tokens::*};
use crate::{Span, Tagged, Text};
use derive_new::new;
use enum_utils::FromStr;
use getset::Getters;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum TokenNode {
    Token(Token),
    #[allow(unused)]
    Call(Tagged<CallNode>),
    Delimited(Tagged<DelimitedNode>),
    Pipeline(Tagged<Pipeline>),
    Operator(Tagged<Operator>),
    Flag(Tagged<Flag>),
    Member(Span),
    Whitespace(Span),
    #[allow(unused)]
    Error(Tagged<Box<ShellError>>),
    Path(Tagged<PathNode>),
}

pub struct DebugTokenNode<'a> {
    node: &'a TokenNode,
    source: &'a Text,
}

impl fmt::Debug for DebugTokenNode<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.node {
            TokenNode::Token(t) => write!(f, "{:?}", t.debug(self.source)),
            TokenNode::Call(s) => {
                write!(f, "(")?;

                write!(f, "{:?}", s.head().debug(self.source))?;

                if let Some(children) = s.children() {
                    for child in children {
                        write!(f, "{:?}", child.debug(self.source))?;
                    }
                }

                write!(f, ")")
            }

            TokenNode::Delimited(d) => {
                write!(
                    f,
                    "{}",
                    match d.delimiter {
                        Delimiter::Brace => "{",
                        Delimiter::Paren => "(",
                        Delimiter::Square => "[",
                    }
                )?;

                for child in d.children() {
                    write!(f, "{:?}", child.debug(self.source))?;
                }

                write!(
                    f,
                    "{}",
                    match d.delimiter {
                        Delimiter::Brace => "}",
                        Delimiter::Paren => ")",
                        Delimiter::Square => "]",
                    }
                )
            }
            TokenNode::Pipeline(_) => write!(f, "<todo:pipeline>"),
            TokenNode::Error(s) => write!(f, "<error> for {:?}", s.span().slice(self.source)),
            rest => write!(f, "{}", rest.span().slice(self.source)),
        }
    }
}

impl From<&TokenNode> for Span {
    fn from(token: &TokenNode) -> Span {
        token.span()
    }
}

impl TokenNode {
    pub fn span(&self) -> Span {
        match self {
            TokenNode::Token(t) => t.span(),
            TokenNode::Call(s) => s.span(),
            TokenNode::Delimited(s) => s.span(),
            TokenNode::Pipeline(s) => s.span(),
            TokenNode::Operator(s) => s.span(),
            TokenNode::Flag(s) => s.span(),
            TokenNode::Member(s) => *s,
            TokenNode::Whitespace(s) => *s,
            TokenNode::Error(s) => s.span(),
            TokenNode::Path(s) => s.span(),
        }
    }

    pub fn type_name(&self) -> String {
        match self {
            TokenNode::Token(t) => t.type_name(),
            TokenNode::Call(_) => "command",
            TokenNode::Delimited(d) => d.type_name(),
            TokenNode::Pipeline(_) => "pipeline",
            TokenNode::Operator(_) => "operator",
            TokenNode::Flag(_) => "flag",
            TokenNode::Member(_) => "member",
            TokenNode::Whitespace(_) => "whitespace",
            TokenNode::Error(_) => "error",
            TokenNode::Path(_) => "path",
        }
        .to_string()
    }

    pub fn debug<'a>(&'a self, source: &'a Text) -> DebugTokenNode<'a> {
        DebugTokenNode { node: self, source }
    }

    pub fn as_external_arg(&self, source: &Text) -> String {
        self.span().slice(source).to_string()
    }

    pub fn source<'a>(&self, source: &'a Text) -> &'a str {
        self.span().slice(source)
    }

    pub fn is_bare(&self) -> bool {
        match self {
            TokenNode::Token(Tagged {
                item: RawToken::Bare,
                ..
            }) => true,
            _ => false,
        }
    }

    pub fn is_external(&self) -> bool {
        match self {
            TokenNode::Token(Tagged {
                item: RawToken::External(..),
                ..
            }) => true,
            _ => false,
        }
    }

    pub fn expect_external(&self) -> Span {
        match self {
            TokenNode::Token(Tagged {
                item: RawToken::External(span),
                ..
            }) => *span,
            _ => panic!("Only call expect_external if you checked is_external first"),
        }
    }

    pub(crate) fn as_flag(&self, value: &str, source: &Text) -> Option<Tagged<Flag>> {
        match self {
            TokenNode::Flag(
                flag @ Tagged {
                    item: Flag { .. }, ..
                },
            ) if value == flag.name().slice(source) => Some(*flag),
            _ => None,
        }
    }

    pub fn as_pipeline(&self) -> Result<Pipeline, ShellError> {
        match self {
            TokenNode::Pipeline(Tagged { item, .. }) => Ok(item.clone()),
            _ => Err(ShellError::string("unimplemented")),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Getters, new)]
#[get = "pub(crate)"]
pub struct DelimitedNode {
    delimiter: Delimiter,
    children: Vec<TokenNode>,
}

impl DelimitedNode {
    pub fn type_name(&self) -> &'static str {
        match self.delimiter {
            Delimiter::Brace => "braced expression",
            Delimiter::Paren => "parenthesized expression",
            Delimiter::Square => "array literal or index operator",
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, FromStr)]
pub enum Delimiter {
    Paren,
    Brace,
    Square,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Getters, new)]
#[get = "pub(crate)"]
pub struct PathNode {
    head: Box<TokenNode>,
    tail: Vec<TokenNode>,
}
