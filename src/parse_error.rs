pub use crate::Span;

#[derive(Debug)]
pub enum ParseError {
    ExtraTokens(Span),
    UnexpectedEof(String, Span),
    UnknownStatement(Span),
    Mismatch(String, Span),
    VariableNotFound(Span),
    UnknownCommand(Span),
}
