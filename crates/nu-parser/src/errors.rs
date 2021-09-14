use nu_protocol::{Span, Type};

#[derive(Debug)]
pub enum ParseError {
    ExtraTokens(Span),
    ExtraPositional(Span),
    UnexpectedEof(String, Span),
    Unclosed(String, Span),
    UnknownStatement(Span),
    Expected(String, Span),
    Mismatch(String, String, Span), // expected, found, span
    UnsupportedOperation(Span, Span, Type, Span, Type),
    ExpectedKeyword(String, Span),
    MultipleRestParams(Span),
    VariableNotFound(Span),
    UnknownCommand(Span),
    NonUtf8(Span),
    UnknownFlag(Span),
    UnknownType(Span),
    MissingFlagParam(Span),
    ShortFlagBatchCantTakeArg(Span),
    MissingPositional(String, Span),
    KeywordMissingArgument(String, Span),
    MissingType(Span),
    TypeMismatch(Type, Type, Span), // expected, found, span
    MissingRequiredFlag(String, Span),
    IncompleteMathExpression(Span),
    UnknownState(String, Span),
    IncompleteParser(Span),
    RestNeedsName(Span),
    ExtraColumns(usize, Span),
    MissingColumns(usize, Span),
    AssignmentMismatch(String, String, Span),
}
