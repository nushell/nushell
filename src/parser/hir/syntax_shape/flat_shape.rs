use crate::parser::{Delimiter, Flag, FlagKind, Operator, RawNumber, RawToken, TokenNode};
use crate::{Span, Spanned, SpannedItem, Text};

#[derive(Debug, Copy, Clone)]
pub enum FlatShape {
    OpenDelimiter(Delimiter),
    CloseDelimiter(Delimiter),
    ItVariable,
    Variable,
    Operator,
    Dot,
    InternalCommand,
    ExternalCommand,
    ExternalWord,
    BareMember,
    StringMember,
    String,
    Path,
    Word,
    Pipe,
    GlobPattern,
    Flag,
    ShorthandFlag,
    Int,
    Decimal,
    Whitespace,
    Error,
    Size { number: Span, unit: Span },
}

impl FlatShape {
    pub fn from(token: &TokenNode, source: &Text, shapes: &mut Vec<Spanned<FlatShape>>) -> () {
        match token {
            TokenNode::Token(token) => match token.item {
                RawToken::Number(RawNumber::Int(_)) => {
                    shapes.push(FlatShape::Int.spanned(token.span))
                }
                RawToken::Number(RawNumber::Decimal(_)) => {
                    shapes.push(FlatShape::Decimal.spanned(token.span))
                }
                RawToken::Operator(Operator::Dot) => {
                    shapes.push(FlatShape::Dot.spanned(token.span))
                }
                RawToken::Operator(_) => shapes.push(FlatShape::Operator.spanned(token.span)),
                RawToken::String(_) => shapes.push(FlatShape::String.spanned(token.span)),
                RawToken::Variable(v) if v.slice(source) == "it" => {
                    shapes.push(FlatShape::ItVariable.spanned(token.span))
                }
                RawToken::Variable(_) => shapes.push(FlatShape::Variable.spanned(token.span)),
                RawToken::ExternalCommand(_) => {
                    shapes.push(FlatShape::ExternalCommand.spanned(token.span))
                }
                RawToken::ExternalWord => shapes.push(FlatShape::ExternalWord.spanned(token.span)),
                RawToken::GlobPattern => shapes.push(FlatShape::GlobPattern.spanned(token.span)),
                RawToken::Bare => shapes.push(FlatShape::Word.spanned(token.span)),
            },
            TokenNode::Call(_) => unimplemented!(),
            TokenNode::Nodes(nodes) => {
                for node in &nodes.item {
                    FlatShape::from(node, source, shapes);
                }
            }
            TokenNode::Delimited(v) => {
                shapes.push(FlatShape::OpenDelimiter(v.item.delimiter).spanned(v.item.spans.0));
                for token in &v.item.children {
                    FlatShape::from(token, source, shapes);
                }
                shapes.push(FlatShape::CloseDelimiter(v.item.delimiter).spanned(v.item.spans.1));
            }
            TokenNode::Pipeline(pipeline) => {
                for part in &pipeline.parts {
                    if let Some(_) = part.pipe {
                        shapes.push(FlatShape::Pipe.spanned(part.span));
                    }
                }
            }
            TokenNode::Flag(Spanned {
                item:
                    Flag {
                        kind: FlagKind::Longhand,
                        ..
                    },
                span,
            }) => shapes.push(FlatShape::Flag.spanned(*span)),
            TokenNode::Flag(Spanned {
                item:
                    Flag {
                        kind: FlagKind::Shorthand,
                        ..
                    },
                span,
            }) => shapes.push(FlatShape::ShorthandFlag.spanned(*span)),
            TokenNode::Whitespace(_) => shapes.push(FlatShape::Whitespace.spanned(token.span())),
            TokenNode::Error(v) => shapes.push(FlatShape::Error.spanned(v.span)),
        }
    }
}
