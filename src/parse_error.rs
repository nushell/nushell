pub use crate::Span;

#[derive(Debug)]
pub enum ParseError {
    ExtraTokens(Span),
    ExtraPositional(Span),
    UnexpectedEof(String, Span),
    Unclosed(String, Span),
    UnknownStatement(Span),
    Mismatch(String, Span),
    VariableNotFound(Span),
    UnknownCommand(Span),
    NonUtf8(Span),
    UnknownFlag(Span),
    MissingFlagParam(Span),
    ShortFlagBatchCantTakeArg(Span),
    MissingPositional(String, Span),
    MissingType(Span),
    MissingRequiredFlag(String, Span),
    IncompleteMathExpression(Span),
    UnknownState(String, Span),
}
