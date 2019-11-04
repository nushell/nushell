use crate::errors::{ParseError, ShellError};
use crate::parser::parse::{call_node::*, flag::*, operator::*, pipeline::*, tokens::*};
use crate::prelude::*;
use crate::traits::ToDebug;
use crate::{Tagged, Text};
use derive_new::new;
use getset::Getters;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum TokenNode {
    Token(Token),

    Call(Spanned<CallNode>),
    Nodes(Spanned<Vec<TokenNode>>),
    Delimited(Spanned<DelimitedNode>),
    Pipeline(Spanned<Pipeline>),
    Flag(Spanned<Flag>),
    Whitespace(Span),

    Error(Spanned<ShellError>),
}

impl HasSpan for TokenNode {
    fn span(&self) -> Span {
        self.get_span()
    }
}

impl FormatDebug for TokenNode {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result {
        write!(f, "{:?}", self.old_debug(&Text::from(source)))
    }
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

                write!(f, "{}", s.head().debug(self.source))?;

                if let Some(children) = s.children() {
                    for child in children {
                        write!(f, "{}", child.debug(self.source))?;
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
                    write!(f, "{:?}", child.old_debug(self.source))?;
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
            TokenNode::Pipeline(pipeline) => write!(f, "{}", pipeline.debug(self.source)),
            TokenNode::Error(_) => write!(f, "<error>"),
            rest => write!(f, "{}", rest.span().slice(self.source)),
        }
    }
}

impl From<&TokenNode> for Span {
    fn from(token: &TokenNode) -> Span {
        token.get_span()
    }
}

impl TokenNode {
    pub fn get_span(&self) -> Span {
        match self {
            TokenNode::Token(t) => t.span,
            TokenNode::Nodes(t) => t.span,
            TokenNode::Call(s) => s.span,
            TokenNode::Delimited(s) => s.span,
            TokenNode::Pipeline(s) => s.span,
            TokenNode::Flag(s) => s.span,
            TokenNode::Whitespace(s) => *s,
            TokenNode::Error(s) => s.span,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            TokenNode::Token(t) => t.type_name(),
            TokenNode::Nodes(_) => "nodes",
            TokenNode::Call(_) => "command",
            TokenNode::Delimited(d) => d.type_name(),
            TokenNode::Pipeline(_) => "pipeline",
            TokenNode::Flag(_) => "flag",
            TokenNode::Whitespace(_) => "whitespace",
            TokenNode::Error(_) => "error",
        }
    }

    pub fn spanned_type_name(&self) -> Spanned<&'static str> {
        self.type_name().spanned(self.span())
    }

    pub fn tagged_type_name(&self) -> Tagged<&'static str> {
        self.type_name().tagged(self.span())
    }

    pub fn old_debug<'a>(&'a self, source: &'a Text) -> DebugTokenNode<'a> {
        DebugTokenNode { node: self, source }
    }

    pub fn as_external_arg(&self, source: &Text) -> String {
        self.span().slice(source).to_string()
    }

    pub fn source<'a>(&self, source: &'a Text) -> &'a str {
        self.span().slice(source)
    }

    pub fn get_variable(&self) -> Result<(Span, Span), ShellError> {
        match self {
            TokenNode::Token(Spanned {
                item: RawToken::Variable(inner_span),
                span: outer_span,
            }) => Ok((*outer_span, *inner_span)),
            _ => Err(ShellError::type_error(
                "variable",
                self.type_name().spanned(self.span()),
            )),
        }
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

    pub fn is_string(&self) -> bool {
        match self {
            TokenNode::Token(Spanned {
                item: RawToken::String(_),
                ..
            }) => true,
            _ => false,
        }
    }

    pub fn is_number(&self) -> bool {
        match self {
            TokenNode::Token(Spanned {
                item: RawToken::Number(_),
                ..
            }) => true,
            _ => false,
        }
    }

    pub fn as_string(&self) -> Option<(Span, Span)> {
        match self {
            TokenNode::Token(Spanned {
                item: RawToken::String(inner_span),
                span: outer_span,
            }) => Some((*outer_span, *inner_span)),
            _ => None,
        }
    }

    pub fn is_pattern(&self) -> bool {
        match self {
            TokenNode::Token(Spanned {
                item: RawToken::GlobPattern,
                ..
            }) => true,
            _ => false,
        }
    }

    pub fn is_word(&self) -> bool {
        match self {
            TokenNode::Token(Spanned {
                item: RawToken::Bare,
                ..
            }) => true,
            _ => false,
        }
    }

    pub fn is_int(&self) -> bool {
        match self {
            TokenNode::Token(Spanned {
                item: RawToken::Number(RawNumber::Int(_)),
                ..
            }) => true,
            _ => false,
        }
    }

    pub fn is_dot(&self) -> bool {
        match self {
            TokenNode::Token(Spanned {
                item: RawToken::Operator(Operator::Dot),
                ..
            }) => true,
            _ => false,
        }
    }

    pub fn as_block(&self) -> Option<(Spanned<&[TokenNode]>, (Span, Span))> {
        match self {
            TokenNode::Delimited(Spanned {
                item:
                    DelimitedNode {
                        delimiter,
                        children,
                        spans,
                    },
                span,
            }) if *delimiter == Delimiter::Brace => Some(((&children[..]).spanned(*span), *spans)),
            _ => None,
        }
    }

    pub fn is_external(&self) -> bool {
        match self {
            TokenNode::Token(Spanned {
                item: RawToken::ExternalCommand(..),
                ..
            }) => true,
            _ => false,
        }
    }

    pub(crate) fn as_flag(&self, value: &str, source: &Text) -> Option<Spanned<Flag>> {
        match self {
            TokenNode::Flag(
                flag @ Spanned {
                    item: Flag { .. }, ..
                },
            ) if value == flag.name().slice(source) => Some(*flag),
            _ => None,
        }
    }

    pub fn as_pipeline(&self) -> Result<Pipeline, ParseError> {
        match self {
            TokenNode::Pipeline(Spanned { item, .. }) => Ok(item.clone()),
            other => Err(ParseError::mismatch(
                "pipeline",
                other.type_name().spanned(other.span()),
            )),
        }
    }

    pub fn is_whitespace(&self) -> bool {
        match self {
            TokenNode::Whitespace(_) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Getters, new)]
#[get = "pub(crate)"]
pub struct DelimitedNode {
    pub(crate) delimiter: Delimiter,
    pub(crate) spans: (Span, Span),
    pub(crate) children: Vec<TokenNode>,
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Delimiter {
    Paren,
    Brace,
    Square,
}

impl Delimiter {
    pub(crate) fn open(&self) -> &'static str {
        match self {
            Delimiter::Paren => "(",
            Delimiter::Brace => "{",
            Delimiter::Square => "[",
        }
    }

    pub(crate) fn close(&self) -> &'static str {
        match self {
            Delimiter::Paren => ")",
            Delimiter::Brace => "}",
            Delimiter::Square => "]",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Getters, new)]
#[get = "pub(crate)"]
pub struct PathNode {
    head: Box<TokenNode>,
    tail: Vec<TokenNode>,
}

#[cfg(test)]
impl TokenNode {
    pub fn expect_external(&self) -> Span {
        match self {
            TokenNode::Token(Spanned {
                item: RawToken::ExternalCommand(span),
                ..
            }) => *span,
            other => panic!(
                "Only call expect_external if you checked is_external first, found {:?}",
                other
            ),
        }
    }

    pub fn expect_string(&self) -> (Span, Span) {
        match self {
            TokenNode::Token(Spanned {
                item: RawToken::String(inner_span),
                span: outer_span,
            }) => (*outer_span, *inner_span),
            other => panic!("Expected string, found {:?}", other),
        }
    }

    pub fn expect_list(&self) -> Spanned<&[TokenNode]> {
        match self {
            TokenNode::Nodes(token_nodes) => token_nodes[..].spanned(token_nodes.span),
            other => panic!("Expected list, found {:?}", other),
        }
    }

    pub fn expect_pattern(&self) -> Span {
        match self {
            TokenNode::Token(Spanned {
                item: RawToken::GlobPattern,
                span: outer_span,
            }) => *outer_span,
            other => panic!("Expected pattern, found {:?}", other),
        }
    }

    pub fn expect_var(&self) -> (Span, Span) {
        match self {
            TokenNode::Token(Spanned {
                item: RawToken::Variable(inner_span),
                span: outer_span,
            }) => (*outer_span, *inner_span),
            other => panic!("Expected var, found {:?}", other),
        }
    }

    pub fn expect_dot(&self) -> Span {
        match self {
            TokenNode::Token(Spanned {
                item: RawToken::Operator(Operator::Dot),
                span,
            }) => *span,
            other => panic!("Expected dot, found {:?}", other),
        }
    }

    pub fn expect_bare(&self) -> Span {
        match self {
            TokenNode::Token(Spanned {
                item: RawToken::Bare,
                span,
            }) => *span,
            other => panic!("Expected bare, found {:?}", other),
        }
    }
}
