use crate::parse::flag::{Flag, FlagKind};
use crate::parse::operator::EvaluationOperator;
use crate::parse::token_tree::{Delimiter, TokenNode};
use crate::parse::tokens::{RawNumber, UnspannedToken};
use nu_source::{HasSpan, Span, Spanned, SpannedItem, Text};

#[derive(Debug, Copy, Clone)]
pub enum FlatShape {
    OpenDelimiter(Delimiter),
    CloseDelimiter(Delimiter),
    ItVariable,
    Variable,
    CompareOperator,
    Dot,
    DotDot,
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
    Separator,
    Error,
    Comment,
    Size { number: Span, unit: Span },
}

impl FlatShape {
    pub fn from(token: &TokenNode, source: &Text, shapes: &mut Vec<Spanned<FlatShape>>) {
        match token {
            TokenNode::Token(token) => match token.unspanned {
                UnspannedToken::Number(RawNumber::Int(_)) => {
                    shapes.push(FlatShape::Int.spanned(token.span))
                }
                UnspannedToken::Number(RawNumber::Decimal(_)) => {
                    shapes.push(FlatShape::Decimal.spanned(token.span))
                }
                UnspannedToken::EvaluationOperator(EvaluationOperator::Dot) => {
                    shapes.push(FlatShape::Dot.spanned(token.span))
                }
                UnspannedToken::EvaluationOperator(EvaluationOperator::DotDot) => {
                    shapes.push(FlatShape::DotDot.spanned(token.span))
                }
                UnspannedToken::CompareOperator(_) => {
                    shapes.push(FlatShape::CompareOperator.spanned(token.span))
                }
                UnspannedToken::String(_) => shapes.push(FlatShape::String.spanned(token.span)),
                UnspannedToken::Variable(v) if v.slice(source) == "it" => {
                    shapes.push(FlatShape::ItVariable.spanned(token.span))
                }
                UnspannedToken::Variable(_) => shapes.push(FlatShape::Variable.spanned(token.span)),
                UnspannedToken::ExternalCommand(_) => {
                    shapes.push(FlatShape::ExternalCommand.spanned(token.span))
                }
                UnspannedToken::ExternalWord => {
                    shapes.push(FlatShape::ExternalWord.spanned(token.span))
                }
                UnspannedToken::GlobPattern => {
                    shapes.push(FlatShape::GlobPattern.spanned(token.span))
                }
                UnspannedToken::Bare => shapes.push(FlatShape::Word.spanned(token.span)),
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
                    if part.pipe.is_some() {
                        shapes.push(FlatShape::Pipe.spanned(part.span()));
                    }
                }
            }
            TokenNode::Flag(Flag {
                kind: FlagKind::Longhand,
                span,
                ..
            }) => shapes.push(FlatShape::Flag.spanned(*span)),
            TokenNode::Flag(Flag {
                kind: FlagKind::Shorthand,
                span,
                ..
            }) => shapes.push(FlatShape::ShorthandFlag.spanned(*span)),
            TokenNode::Whitespace(_) => shapes.push(FlatShape::Whitespace.spanned(token.span())),
            TokenNode::Separator(_) => shapes.push(FlatShape::Separator.spanned(token.span())),
            TokenNode::Comment(_) => shapes.push(FlatShape::Comment.spanned(token.span())),
            TokenNode::Error(v) => shapes.push(FlatShape::Error.spanned(v.span)),
        }
    }
}
