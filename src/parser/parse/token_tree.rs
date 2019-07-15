use crate::errors::ShellError;
use crate::parser::parse::{call_node::*, flag::*, operator::*, pipeline::*, span::*, tokens::*};
use crate::Text;
use derive_new::new;
use enum_utils::FromStr;
use getset::Getters;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum TokenNode {
    Token(Token),
    #[allow(unused)]
    Call(Spanned<CallNode>),
    Delimited(Spanned<DelimitedNode>),
    Pipeline(Spanned<Pipeline>),
    Operator(Spanned<Operator>),
    Flag(Spanned<Flag>),
    Member(Span),
    Whitespace(Span),
    #[allow(unused)]
    Error(Spanned<Box<ShellError>>),
    Path(Spanned<PathNode>),
}

pub struct DebugTokenNode<'a> {
    node: &'a TokenNode,
    source: &'a Text,
}

impl fmt::Debug for DebugTokenNode<'a> {
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
            TokenNode::Token(t) => t.span,
            TokenNode::Call(s) => s.span,
            TokenNode::Delimited(s) => s.span,
            TokenNode::Pipeline(s) => s.span,
            TokenNode::Operator(s) => s.span,
            TokenNode::Flag(s) => s.span,
            TokenNode::Member(s) => *s,
            TokenNode::Whitespace(s) => *s,
            TokenNode::Error(s) => s.span,
            TokenNode::Path(s) => s.span,
        }
    }

    pub fn type_name(&self) -> String {
        match self {
            TokenNode::Token(t) => t.type_name(),
            TokenNode::Call(s) => "command",
            TokenNode::Delimited(d) => d.type_name(),
            TokenNode::Pipeline(s) => "pipeline",
            TokenNode::Operator(s) => "operator",
            TokenNode::Flag(s) => "flag",
            TokenNode::Member(s) => "member",
            TokenNode::Whitespace(s) => "whitespace",
            TokenNode::Error(s) => "error",
            TokenNode::Path(s) => "path",
        }
        .to_string()
    }

    pub fn debug(&'a self, source: &'a Text) -> DebugTokenNode<'a> {
        DebugTokenNode { node: self, source }
    }

    pub fn as_external_arg(&self, source: &Text) -> String {
        self.span().slice(source).to_string()
    }

    pub fn source(&self, source: &'a Text) -> &'a str {
        self.span().slice(source)
    }

    pub fn is_bare(&self) -> bool {
        match self {
            TokenNode::Token(Spanned {
                item: RawToken::Bare,
                ..
            }) => true,
            _ => false,
        }
    }

    crate fn as_flag(&self, value: &str, source: &Text) -> Option<Spanned<Flag>> {
        match self {
            TokenNode::Flag(
                flag @ Spanned {
                    item: Flag { .. }, ..
                },
            ) if value == flag.name().slice(source) => Some(*flag),
            _ => None,
        }
    }

    pub fn as_pipeline(&self) -> Result<Pipeline, ShellError> {
        match self {
            TokenNode::Pipeline(Spanned { item, .. }) => Ok(item.clone()),
            _ => Err(ShellError::string("unimplemented")),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Getters, new)]
#[get = "crate"]
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
#[get = "crate"]
pub struct PathNode {
    head: Box<TokenNode>,
    tail: Vec<TokenNode>,
}
