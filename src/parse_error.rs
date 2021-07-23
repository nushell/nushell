use crate::parser_state::Type;
pub use crate::Span;

#[derive(Debug)]
pub enum ParseError {
    ExtraTokens(Span),
    ExtraPositional(Span),
    UnexpectedEof(String, Span),
    Unclosed(String, Span),
    UnknownStatement(Span),
    Mismatch(String, Span),
    MultipleRestParams(Span),
    VariableNotFound(Span),
    UnknownCommand(Span),
    NonUtf8(Span),
    UnknownFlag(Span),
    UnknownType(Span),
    MissingFlagParam(Span),
    ShortFlagBatchCantTakeArg(Span),
    MissingPositional(String, Span),
    MissingType(Span),
    TypeMismatch(Type, Span),
    MissingRequiredFlag(String, Span),
    IncompleteMathExpression(Span),
    UnknownState(String, Span),
}
