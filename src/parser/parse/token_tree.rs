use crate::errors::ShellError;
use crate::parser::parse::{call_node::*, flag::*, operator::*, pipeline::*, tokens::*};
use crate::prelude::*;
use crate::traits::ToDebug;
use crate::{Tag, Tagged, Text};
use derive_new::new;
use enum_utils::FromStr;
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

impl ToDebug for TokenNode {
    fn fmt_debug(&self, f: &mut fmt::Formatter, source: &str) -> fmt::Result {
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
            rest => write!(f, "{}", rest.tag().slice(self.source)),
        }
    }
}

impl From<&TokenNode> for Tag {
    fn from(token: &TokenNode) -> Tag {
        token.tag()
    }
}

impl TokenNode {
    pub fn tag(&self) -> Tag {
        match self {
            TokenNode::Token(t) => t.tag(),
            TokenNode::Nodes(t) => Tag {
                span: t.span,
                anchor: uuid::Uuid::nil(),
            },
            TokenNode::Call(s) => Tag {
                span: s.span,
                anchor: uuid::Uuid::nil(),
            },
            TokenNode::Delimited(s) => Tag {
                span: s.span,
                anchor: uuid::Uuid::nil(),
            },
            TokenNode::Pipeline(s) => Tag {
                span: s.span,
                anchor: uuid::Uuid::nil(),
            },
            TokenNode::Flag(s) => Tag {
                span: s.span,
                anchor: uuid::Uuid::nil(),
            },
            TokenNode::Whitespace(s) => Tag {
                span: *s,
                anchor: uuid::Uuid::nil(),
            },
            TokenNode::Error(s) => Tag {
                span: s.span,
                anchor: uuid::Uuid::nil(),
            },
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

    pub fn tagged_type_name(&self) -> Tagged<&'static str> {
        self.type_name().tagged(self.tag())
    }

    pub fn old_debug<'a>(&'a self, source: &'a Text) -> DebugTokenNode<'a> {
        DebugTokenNode { node: self, source }
    }

    pub fn as_external_arg(&self, source: &Text) -> String {
        self.tag().slice(source).to_string()
    }

    pub fn source<'a>(&self, source: &'a Text) -> &'a str {
        self.tag().slice(source)
    }

    pub fn get_variable(&self) -> Result<(Tag, Tag), ShellError> {
        match self {
            TokenNode::Token(Tagged {
                item: RawToken::Variable(inner_span),
                tag: outer_tag,
            }) => Ok((
                *outer_tag,
                Tag {
                    span: *inner_span,
                    anchor: uuid::Uuid::nil(),
                },
            )),
            _ => Err(ShellError::type_error("variable", self.tagged_type_name())),
        }
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

    pub fn is_pattern(&self) -> bool {
        match self {
            TokenNode::Token(Tagged {
                item: RawToken::GlobPattern,
                ..
            }) => true,
            _ => false,
        }
    }

    pub fn is_dot(&self) -> bool {
        match self {
            TokenNode::Token(Tagged {
                item: RawToken::Operator(Operator::Dot),
                ..
            }) => true,
            _ => false,
        }
    }

    pub fn as_block(&self) -> Option<(Spanned<&[TokenNode]>, (Tag, Tag))> {
        match self {
            TokenNode::Delimited(Spanned {
                item:
                    DelimitedNode {
                        delimiter,
                        children,
                        tags,
                    },
                span,
            }) if *delimiter == Delimiter::Brace => Some(((&children[..]).spanned(*span), *tags)),
            _ => None,
        }
    }

    pub fn is_external(&self) -> bool {
        match self {
            TokenNode::Token(Tagged {
                item: RawToken::ExternalCommand(..),
                ..
            }) => true,
            _ => false,
        }
    }

    pub fn expect_external(&self) -> Tag {
        match self {
            TokenNode::Token(Tagged {
                item: RawToken::ExternalCommand(span),
                ..
            }) => Tag {
                span: *span,
                anchor: uuid::Uuid::nil(),
            },
            _ => panic!("Only call expect_external if you checked is_external first"),
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

    pub fn as_pipeline(&self) -> Result<Pipeline, ShellError> {
        match self {
            TokenNode::Pipeline(Spanned { item, .. }) => Ok(item.clone()),
            _ => Err(ShellError::unimplemented("unimplemented")),
        }
    }

    pub fn is_whitespace(&self) -> bool {
        match self {
            TokenNode::Whitespace(_) => true,
            _ => false,
        }
    }

    pub fn expect_string(&self) -> (Span, Span) {
        match self {
            TokenNode::Token(Tagged {
                item: RawToken::String(inner_span),
                tag: outer_tag,
            }) => (outer_tag.span, *inner_span),
            other => panic!("Expected string, found {:?}", other),
        }
    }
}

#[cfg(test)]
impl TokenNode {
    pub fn expect_list(&self) -> Tagged<&[TokenNode]> {
        match self {
            TokenNode::Nodes(Spanned { item, span }) => (&item[..]).tagged(Tag {
                span: *span,
                anchor: uuid::Uuid::nil(),
            }),
            other => panic!("Expected list, found {:?}", other),
        }
    }

    pub fn expect_var(&self) -> (Tag, Tag) {
        match self {
            TokenNode::Token(Tagged {
                item: RawToken::Variable(inner_span),
                tag: outer_tag,
            }) => (
                *outer_tag,
                Tag {
                    span: *inner_span,
                    anchor: uuid::Uuid::nil(),
                },
            ),
            other => panic!("Expected var, found {:?}", other),
        }
    }

    pub fn expect_bare(&self) -> Tag {
        match self {
            TokenNode::Token(Tagged {
                item: RawToken::Bare,
                tag,
            }) => *tag,
            other => panic!("Expected var, found {:?}", other),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Getters, new)]
#[get = "pub(crate)"]
pub struct DelimitedNode {
    pub(crate) delimiter: Delimiter,
    pub(crate) tags: (Tag, Tag),
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, FromStr)]
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
