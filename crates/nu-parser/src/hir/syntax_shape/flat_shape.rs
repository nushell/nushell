use crate::parse::flag::{Flag, FlagKind};
use crate::parse::number::RawNumber;
use crate::parse::operator::EvaluationOperator;
use crate::parse::token_tree::{Delimiter, SpannedToken, Token};
use nu_protocol::ShellTypeName;
use nu_source::{DebugDocBuilder, HasSpan, PrettyDebug, Span, Spanned, SpannedItem, Text};

#[derive(Debug, Copy, Clone)]
pub enum FlatShape {
    OpenDelimiter(Delimiter),
    CloseDelimiter(Delimiter),
    Type,
    Identifier,
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
    Keyword,
    Pipe,
    GlobPattern,
    Flag,
    ShorthandFlag,
    Int,
    Decimal,
    Whitespace,
    Separator,
    Comment,
    Size { number: Span, unit: Span },
}

#[derive(Debug, Clone)]
pub enum ShapeResult {
    Success(Spanned<FlatShape>),
    Fallback {
        shape: Spanned<FlatShape>,
        allowed: Vec<String>,
    },
}

impl HasSpan for ShapeResult {
    fn span(&self) -> Span {
        match self {
            ShapeResult::Success(shape) => shape.span,
            ShapeResult::Fallback { shape, .. } => shape.span,
        }
    }
}

impl PrettyDebug for FlatShape {
    fn pretty(&self) -> DebugDocBuilder {
        unimplemented!()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct TraceShape {
    shape: FlatShape,
    span: Span,
}

impl ShellTypeName for TraceShape {
    fn type_name(&self) -> &'static str {
        self.shape.type_name()
    }
}

impl PrettyDebug for TraceShape {
    fn pretty(&self) -> DebugDocBuilder {
        self.shape.pretty()
    }
}

impl HasSpan for TraceShape {
    fn span(&self) -> Span {
        self.span
    }
}

impl ShellTypeName for FlatShape {
    fn type_name(&self) -> &'static str {
        match self {
            FlatShape::OpenDelimiter(Delimiter::Brace) => "open brace",
            FlatShape::OpenDelimiter(Delimiter::Paren) => "open paren",
            FlatShape::OpenDelimiter(Delimiter::Square) => "open square",
            FlatShape::CloseDelimiter(Delimiter::Brace) => "close brace",
            FlatShape::CloseDelimiter(Delimiter::Paren) => "close paren",
            FlatShape::CloseDelimiter(Delimiter::Square) => "close square",
            FlatShape::Type => "type",
            FlatShape::Identifier => "identifier",
            FlatShape::ItVariable => "$it",
            FlatShape::Variable => "variable",
            FlatShape::CompareOperator => "comparison",
            FlatShape::Dot => "dot",
            FlatShape::DotDot => "dotdot",
            FlatShape::InternalCommand => "internal command",
            FlatShape::ExternalCommand => "external command",
            FlatShape::ExternalWord => "external word",
            FlatShape::BareMember => "bare member",
            FlatShape::StringMember => "string member",
            FlatShape::String => "string",
            FlatShape::Path => "path",
            FlatShape::Word => "word",
            FlatShape::Keyword => "keyword",
            FlatShape::Pipe => "pipe",
            FlatShape::GlobPattern => "glob",
            FlatShape::Flag => "flag",
            FlatShape::ShorthandFlag => "shorthand flag",
            FlatShape::Int => "int",
            FlatShape::Decimal => "decimal",
            FlatShape::Whitespace => "whitespace",
            FlatShape::Separator => "separator",
            FlatShape::Comment => "comment",
            FlatShape::Size { .. } => "size",
        }
    }
}

impl FlatShape {
    pub fn into_trace_shape(self, span: Span) -> TraceShape {
        TraceShape { shape: self, span }
    }

    pub fn shapes(token: &SpannedToken, source: &Text) -> Vec<Spanned<FlatShape>> {
        let mut shapes = vec![];

        FlatShape::from(token, source, &mut shapes);
        shapes
    }

    fn from(token: &SpannedToken, source: &Text, shapes: &mut Vec<Spanned<FlatShape>>) -> () {
        let span = token.span();

        match token.unspanned() {
            Token::Number(RawNumber::Int(_)) => shapes.push(FlatShape::Int.spanned(span)),
            Token::Number(RawNumber::Decimal(_)) => shapes.push(FlatShape::Decimal.spanned(span)),
            Token::EvaluationOperator(EvaluationOperator::Dot) => {
                shapes.push(FlatShape::Dot.spanned(span))
            }
            Token::EvaluationOperator(EvaluationOperator::DotDot) => {
                shapes.push(FlatShape::DotDot.spanned(span))
            }
            Token::CompareOperator(_) => shapes.push(FlatShape::CompareOperator.spanned(span)),
            Token::String(_) => shapes.push(FlatShape::String.spanned(span)),
            Token::Variable(v) if v.slice(source) == "it" => {
                shapes.push(FlatShape::ItVariable.spanned(span))
            }
            Token::Variable(_) => shapes.push(FlatShape::Variable.spanned(span)),
            Token::ItVariable(_) => shapes.push(FlatShape::ItVariable.spanned(span)),
            Token::ExternalCommand(_) => shapes.push(FlatShape::ExternalCommand.spanned(span)),
            Token::ExternalWord => shapes.push(FlatShape::ExternalWord.spanned(span)),
            Token::GlobPattern => shapes.push(FlatShape::GlobPattern.spanned(span)),
            Token::Bare => shapes.push(FlatShape::Word.spanned(span)),
            Token::Call(_) => unimplemented!(),
            Token::Delimited(v) => {
                shapes.push(FlatShape::OpenDelimiter(v.delimiter).spanned(v.spans.0));
                for token in &v.children {
                    FlatShape::from(token, source, shapes);
                }
                shapes.push(FlatShape::CloseDelimiter(v.delimiter).spanned(v.spans.1));
            }
            Token::Pipeline(pipeline) => {
                for part in &pipeline.parts {
                    if let Some(_) = part.pipe {
                        shapes.push(FlatShape::Pipe.spanned(part.span()));
                    }
                }
            }
            Token::Flag(Flag {
                kind: FlagKind::Longhand,
                ..
            }) => shapes.push(FlatShape::Flag.spanned(span)),
            Token::Flag(Flag {
                kind: FlagKind::Shorthand,
                ..
            }) => shapes.push(FlatShape::ShorthandFlag.spanned(span)),
            Token::Whitespace => shapes.push(FlatShape::Whitespace.spanned(span)),
            Token::Separator => shapes.push(FlatShape::Separator.spanned(span)),
            Token::Comment(_) => shapes.push(FlatShape::Comment.spanned(span)),
        }
    }
}
