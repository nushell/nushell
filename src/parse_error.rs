pub use crate::Span;

#[derive(Debug)]
pub enum ParseError {
    ExtraTokens(Span),
    ExtraPositional(Span),
    UnexpectedEof(String, Span),
    UnknownStatement(Span),
    Mismatch(String, Span),
    VariableNotFound(Span),
    UnknownCommand(Span),
    NonUtf8(Span),
    UnknownFlag(Span),
    MissingFlagParam(Span),
    ShortFlagBatchCantTakeArg(Span),
}
