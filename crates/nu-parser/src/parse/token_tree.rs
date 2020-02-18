#![allow(clippy::type_complexity)]
use crate::parse::{call_node::*, comment::*, flag::*, number::*, operator::*, pipeline::*};
use derive_new::new;
use getset::Getters;
use nu_errors::{ParseError, ShellError};
use nu_protocol::{ShellTypeName, SpannedTypeName};
use nu_source::{
    b, DebugDocBuilder, HasSpan, PrettyDebugWithSource, Span, Spanned, SpannedItem, Text,
};
use std::borrow::Cow;
use std::ops::Deref;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Token {
    Number(RawNumber),
    CompareOperator(CompareOperator),
    EvaluationOperator(EvaluationOperator),
    String(Span),
    Variable(Span),
    ItVariable(Span),
    ExternalCommand(Span),
    ExternalWord,
    GlobPattern,
    Bare,
    Garbage,

    Call(CallNode),
    Delimited(DelimitedNode),
    Pipeline(Pipeline),
    Flag(Flag),
    Comment(Comment),
    Whitespace,
    Separator,
}

macro_rules! token_type {
    (struct $name:tt (desc: $desc:tt) -> $out:ty { |$span:ident, $pat:pat| => $do:expr }) => {
        pub struct $name;

        impl TokenType for $name {
            type Output = $out;

            fn desc(&self) -> Cow<'static, str> {
                Cow::Borrowed($desc)
            }

            fn extract_token_value(
                &self,
                token: &SpannedToken,
                err: ParseErrorFn<$out>,
            ) -> Result<$out, ParseError> {
                let $span = token.span();

                match *token.unspanned() {
                    $pat => Ok($do),
                    _ => err(),
                }
            }
        }
    };

    (struct $name:tt (desc: $desc:tt) -> $out:ty { $pat:pat => $do:expr }) => {
        pub struct $name;

        impl TokenType for $name {
            type Output = $out;

            fn desc(&self) -> Cow<'static, str> {
                Cow::Borrowed($desc)
            }

            fn extract_token_value(
                &self,
                token: &SpannedToken,
                err: ParseErrorFn<$out>,
            ) -> Result<$out, ParseError> {
                match token.unspanned().clone() {
                    $pat => Ok($do),
                    _ => err(),
                }
            }
        }
    };
}

pub type ParseErrorFn<'a, T> = &'a dyn Fn() -> Result<T, ParseError>;

token_type!(struct IntType (desc: "integer") -> RawNumber {
    Token::Number(number @ RawNumber::Int(_)) => number
});

token_type!(struct DecimalType (desc: "decimal") -> RawNumber {
    Token::Number(number @ RawNumber::Decimal(_)) => number
});

token_type!(struct StringType (desc: "string") -> (Span, Span) {
    |outer, Token::String(inner)| => (inner, outer)
});

token_type!(struct BareType (desc: "word") -> Span {
    |span, Token::Bare| => span
});

token_type!(struct DotType (desc: "dot") -> Span {
    |span, Token::EvaluationOperator(EvaluationOperator::Dot)| => span
});

token_type!(struct DotDotType (desc: "dotdot") -> Span {
    |span, Token::EvaluationOperator(EvaluationOperator::DotDot)| => span
});

token_type!(struct CompareOperatorType (desc: "compare operator") -> (Span, CompareOperator) {
    |span, Token::CompareOperator(operator)| => (span, operator)
});

token_type!(struct ExternalWordType (desc: "external word") -> Span {
    |span, Token::ExternalWord| => span
});

token_type!(struct ExternalCommandType (desc: "external command") -> (Span, Span) {
    |outer, Token::ExternalCommand(inner)| => (inner, outer)
});

token_type!(struct CommentType (desc: "comment") -> (Comment, Span) {
    |outer, Token::Comment(comment)| => (comment, outer)
});

token_type!(struct SeparatorType (desc: "separator") -> Span {
    |span, Token::Separator| => span
});

token_type!(struct WhitespaceType (desc: "whitespace") -> Span {
    |span, Token::Whitespace| => span
});

token_type!(struct WordType (desc: "word") -> Span {
    |span, Token::Bare| => span
});

token_type!(struct ItVarType (desc: "$it") -> (Span, Span) {
    |outer, Token::ItVariable(inner)| => (inner, outer)
});

token_type!(struct VarType (desc: "variable") -> (Span, Span) {
    |outer, Token::Variable(inner)| => (inner, outer)
});

token_type!(struct PipelineType (desc: "pipeline") -> Pipeline {
    Token::Pipeline(pipeline) => pipeline
});

token_type!(struct BlockType (desc: "block") -> DelimitedNode {
    Token::Delimited(block @ DelimitedNode { delimiter: Delimiter::Brace, .. }) => block
});

token_type!(struct SquareType (desc: "square") -> DelimitedNode {
    Token::Delimited(square @ DelimitedNode { delimiter: Delimiter::Square, .. }) => square
});

pub trait TokenType {
    type Output;

    fn desc(&self) -> Cow<'static, str>;

    fn extract_token_value(
        &self,
        token: &SpannedToken,
        err: ParseErrorFn<Self::Output>,
    ) -> Result<Self::Output, ParseError>;
}

impl Token {
    pub fn into_spanned(self, span: impl Into<Span>) -> SpannedToken {
        SpannedToken {
            unspanned: self,
            span: span.into(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Getters)]
pub struct SpannedToken {
    #[get = "pub"]
    unspanned: Token,
    span: Span,
}

impl Deref for SpannedToken {
    type Target = Token;
    fn deref(&self) -> &Self::Target {
        &self.unspanned
    }
}

impl HasSpan for SpannedToken {
    fn span(&self) -> Span {
        self.span
    }
}

impl ShellTypeName for SpannedToken {
    fn type_name(&self) -> &'static str {
        self.unspanned.type_name()
    }
}

impl PrettyDebugWithSource for SpannedToken {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match self.unspanned() {
            Token::Number(number) => number.pretty_debug(source),
            Token::CompareOperator(operator) => operator.pretty_debug(source),
            Token::EvaluationOperator(operator) => operator.pretty_debug(source),
            Token::String(_) | Token::GlobPattern | Token::Bare => {
                b::primitive(self.span.slice(source))
            }
            Token::Variable(_) => b::var(self.span.slice(source)),
            Token::ItVariable(_) => b::keyword(self.span.slice(source)),
            Token::ExternalCommand(_) => b::description(self.span.slice(source)),
            Token::ExternalWord => b::description(self.span.slice(source)),
            Token::Call(call) => call.pretty_debug(source),
            Token::Delimited(delimited) => delimited.pretty_debug(source),
            Token::Pipeline(pipeline) => pipeline.pretty_debug(source),
            Token::Flag(flag) => flag.pretty_debug(source),
            Token::Garbage => b::error(self.span.slice(source)),
            Token::Whitespace => b::typed(
                "whitespace",
                b::description(format!("{:?}", self.span.slice(source))),
            ),
            Token::Separator => b::typed(
                "separator",
                b::description(format!("{:?}", self.span.slice(source))),
            ),
            Token::Comment(comment) => {
                b::typed("comment", b::description(comment.text.slice(source)))
            }
        }
    }
}

impl ShellTypeName for Token {
    fn type_name(&self) -> &'static str {
        match self {
            Token::Number(_) => "number",
            Token::CompareOperator(_) => "comparison operator",
            Token::EvaluationOperator(EvaluationOperator::Dot) => "dot",
            Token::EvaluationOperator(EvaluationOperator::DotDot) => "dot dot",
            Token::String(_) => "string",
            Token::Variable(_) => "variable",
            Token::ItVariable(_) => "it variable",
            Token::ExternalCommand(_) => "external command",
            Token::ExternalWord => "external word",
            Token::GlobPattern => "glob pattern",
            Token::Bare => "word",
            Token::Call(_) => "command",
            Token::Delimited(d) => d.type_name(),
            Token::Pipeline(_) => "pipeline",
            Token::Flag(_) => "flag",
            Token::Garbage => "garbage",
            Token::Whitespace => "whitespace",
            Token::Separator => "separator",
            Token::Comment(_) => "comment",
        }
    }
}

impl From<&SpannedToken> for Span {
    fn from(token: &SpannedToken) -> Span {
        token.span
    }
}

impl SpannedToken {
    pub fn as_external_arg(&self, source: &Text) -> String {
        self.span().slice(source).to_string()
    }

    pub fn source<'a>(&self, source: &'a Text) -> &'a str {
        self.span().slice(source)
    }

    pub fn get_variable(&self) -> Result<(Span, Span), ShellError> {
        match self.unspanned() {
            Token::Variable(inner_span) => Ok((self.span(), *inner_span)),
            _ => Err(ShellError::type_error("variable", self.spanned_type_name())),
        }
    }

    pub fn is_bare(&self) -> bool {
        match self.unspanned() {
            Token::Bare => true,
            _ => false,
        }
    }

    pub fn is_string(&self) -> bool {
        match self.unspanned() {
            Token::String(_) => true,
            _ => false,
        }
    }

    pub fn is_number(&self) -> bool {
        match self.unspanned() {
            Token::Number(_) => true,
            _ => false,
        }
    }

    pub fn as_string(&self) -> Option<(Span, Span)> {
        match self.unspanned() {
            Token::String(inner_span) => Some((self.span(), *inner_span)),
            _ => None,
        }
    }

    pub fn is_pattern(&self) -> bool {
        match self.unspanned() {
            Token::GlobPattern => true,
            _ => false,
        }
    }

    pub fn is_word(&self) -> bool {
        match self.unspanned() {
            Token::Bare => true,
            _ => false,
        }
    }

    pub fn is_int(&self) -> bool {
        match self.unspanned() {
            Token::Number(RawNumber::Int(_)) => true,
            _ => false,
        }
    }

    pub fn is_dot(&self) -> bool {
        match self.unspanned() {
            Token::EvaluationOperator(EvaluationOperator::Dot) => true,
            _ => false,
        }
    }

    pub fn as_block(&self) -> Option<(Spanned<&[SpannedToken]>, (Span, Span))> {
        match self.unspanned() {
            Token::Delimited(DelimitedNode {
                delimiter,
                children,
                spans,
            }) if *delimiter == Delimiter::Brace => {
                Some(((&children[..]).spanned(self.span()), *spans))
            }
            _ => None,
        }
    }

    pub fn is_external(&self) -> bool {
        match self.unspanned() {
            Token::ExternalCommand(..) => true,
            _ => false,
        }
    }

    pub(crate) fn as_flag(&self, value: &str, short: Option<char>, source: &Text) -> Option<Flag> {
        match self.unspanned() {
            Token::Flag(flag) => {
                let name = flag.name().slice(source);

                match flag.kind {
                    FlagKind::Longhand if value == name => Some(*flag),
                    FlagKind::Shorthand => {
                        if let Some(short_hand) = short {
                            if short_hand.to_string() == name {
                                return Some(*flag);
                            }
                        }
                        None
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub fn as_pipeline(&self) -> Result<Pipeline, ParseError> {
        match self.unspanned() {
            Token::Pipeline(pipeline) => Ok(pipeline.clone()),
            _ => Err(ParseError::mismatch("pipeline", self.spanned_type_name())),
        }
    }

    pub fn is_whitespace(&self) -> bool {
        match self.unspanned() {
            Token::Whitespace => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Getters, new)]
#[get = "pub(crate)"]
pub struct DelimitedNode {
    pub(crate) delimiter: Delimiter,
    pub(crate) spans: (Span, Span),
    pub(crate) children: Vec<SpannedToken>,
}

impl HasSpan for DelimitedNode {
    fn span(&self) -> Span {
        self.spans.0.until(self.spans.1)
    }
}

impl PrettyDebugWithSource for DelimitedNode {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::delimit(
            self.delimiter.open(),
            b::intersperse(
                self.children.iter().map(|child| child.pretty_debug(source)),
                b::space(),
            ),
            self.delimiter.close(),
        )
    }
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
    pub(crate) fn open(self) -> &'static str {
        match self {
            Delimiter::Paren => "(",
            Delimiter::Brace => "{",
            Delimiter::Square => "[",
        }
    }

    pub(crate) fn close(self) -> &'static str {
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
    head: Box<SpannedToken>,
    tail: Vec<SpannedToken>,
}

#[cfg(test)]
impl SpannedToken {
    pub fn expect_external(&self) -> Span {
        match self.unspanned() {
            Token::ExternalCommand(span) => *span,
            _ => panic!(
                "Only call expect_external if you checked is_external first, found {:?}",
                self
            ),
        }
    }

    pub fn expect_string(&self) -> (Span, Span) {
        match self.unspanned() {
            Token::String(inner_span) => (self.span(), *inner_span),
            other => panic!("Expected string, found {:?}", other),
        }
    }

    pub fn expect_list(&self) -> Spanned<Vec<SpannedToken>> {
        match self.unspanned() {
            Token::Pipeline(pipeline) => pipeline
                .parts()
                .iter()
                .flat_map(|part| part.tokens())
                .cloned()
                .collect::<Vec<SpannedToken>>()
                .spanned(self.span()),
            _ => panic!("Expected list, found {:?}", self),
        }
    }

    pub fn expect_pattern(&self) -> Span {
        match self.unspanned() {
            Token::GlobPattern => self.span(),
            _ => panic!("Expected pattern, found {:?}", self),
        }
    }

    pub fn expect_var(&self) -> (Span, Span) {
        match self.unspanned() {
            Token::Variable(inner_span) => (self.span(), *inner_span),
            Token::ItVariable(inner_span) => (self.span(), *inner_span),
            other => panic!("Expected var, found {:?}", other),
        }
    }

    pub fn expect_dot(&self) -> Span {
        match self.unspanned() {
            Token::EvaluationOperator(EvaluationOperator::Dot) => self.span(),
            other => panic!("Expected dot, found {:?}", other),
        }
    }

    pub fn expect_bare(&self) -> Span {
        match self.unspanned() {
            Token::Bare => self.span(),
            _ => panic!("Expected bare, found {:?}", self),
        }
    }
}
