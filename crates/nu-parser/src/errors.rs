use nu_protocol::SyntaxShape;
use nu_source::Span;

#[derive(Debug, Clone)]
pub enum ParseError {
    UnexpectedEndOfLine(Span),
    UnexpectedFlag(Span),
    NamedArgumentMissingArg(SyntaxShape, Span),
    TooManyPositionalArguments(Span),
    TypeMismatch(SyntaxShape, Span),
    UnknownOperator(Span),
    MissingRequiredPositionalArgument(Span),
    NoShapeMatched(Vec<SyntaxShape>, Span),
}
