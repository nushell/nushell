use crate::parser::{Delimiter, Flag, FlagKind, Operator, RawNumber, RawToken, TokenNode};
use crate::{Tag, Tagged, TaggedItem, Text};

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
    Size { number: Tag, unit: Tag },
}

impl FlatShape {
    pub fn from(token: &TokenNode, source: &Text, shapes: &mut Vec<Tagged<FlatShape>>) -> () {
        match token {
            TokenNode::Token(token) => match token.item {
                RawToken::Number(RawNumber::Int(_)) => {
                    shapes.push(FlatShape::Int.tagged(token.tag))
                }
                RawToken::Number(RawNumber::Decimal(_)) => {
                    shapes.push(FlatShape::Decimal.tagged(token.tag))
                }
                RawToken::Operator(Operator::Dot) => shapes.push(FlatShape::Dot.tagged(token.tag)),
                RawToken::Operator(_) => shapes.push(FlatShape::Operator.tagged(token.tag)),
                RawToken::String(_) => shapes.push(FlatShape::String.tagged(token.tag)),
                RawToken::Variable(v) if v.slice(source) == "it" => {
                    shapes.push(FlatShape::ItVariable.tagged(token.tag))
                }
                RawToken::Variable(_) => shapes.push(FlatShape::Variable.tagged(token.tag)),
                RawToken::ExternalCommand(_) => {
                    shapes.push(FlatShape::ExternalCommand.tagged(token.tag))
                }
                RawToken::ExternalWord => shapes.push(FlatShape::ExternalWord.tagged(token.tag)),
                RawToken::GlobPattern => shapes.push(FlatShape::GlobPattern.tagged(token.tag)),
                RawToken::Bare => shapes.push(FlatShape::Word.tagged(token.tag)),
            },
            TokenNode::Call(_) => unimplemented!(),
            TokenNode::Nodes(nodes) => {
                for node in &nodes.item {
                    FlatShape::from(node, source, shapes);
                }
            }
            TokenNode::Delimited(v) => {
                shapes.push(FlatShape::OpenDelimiter(v.item.delimiter).tagged(v.item.tags.0));
                for token in &v.item.children {
                    FlatShape::from(token, source, shapes);
                }
                shapes.push(FlatShape::CloseDelimiter(v.item.delimiter).tagged(v.item.tags.1));
            }
            TokenNode::Pipeline(pipeline) => {
                for part in &pipeline.parts {
                    if let Some(_) = part.pipe {
                        shapes.push(FlatShape::Pipe.tagged(part.tag));
                    }
                }
            }
            TokenNode::Flag(Tagged {
                item:
                    Flag {
                        kind: FlagKind::Longhand,
                        ..
                    },
                tag,
            }) => shapes.push(FlatShape::Flag.tagged(tag)),
            TokenNode::Flag(Tagged {
                item:
                    Flag {
                        kind: FlagKind::Shorthand,
                        ..
                    },
                tag,
            }) => shapes.push(FlatShape::ShorthandFlag.tagged(tag)),
            TokenNode::Whitespace(_) => shapes.push(FlatShape::Whitespace.tagged(token.tag())),
            TokenNode::Error(v) => shapes.push(FlatShape::Error.tagged(v.tag)),
        }
    }
}
