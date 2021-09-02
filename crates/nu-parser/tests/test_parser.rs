use nu_parser::*;
use nu_parser::{ParseError, ParserState};
use nu_protocol::{Signature, SyntaxShape};

#[test]
pub fn parse_int() {
    let parser_state = ParserState::new();
    let mut working_set = ParserWorkingSet::new(&parser_state);

    let (block, err) = working_set.parse_source(b"3", true);

    assert!(err.is_none());
    assert!(block.len() == 1);
    assert!(matches!(
        block[0],
        Statement::Expression(Expression {
            expr: Expr::Int(3),
            ..
        })
    ));
}

#[test]
pub fn parse_call() {
    let parser_state = ParserState::new();
    let mut working_set = ParserWorkingSet::new(&parser_state);

    let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
    working_set.add_decl(sig.into());

    let (block, err) = working_set.parse_source(b"foo", true);

    assert!(err.is_none());
    assert!(block.len() == 1);

    match &block[0] {
        Statement::Expression(Expression {
            expr: Expr::Call(call),
            ..
        }) => {
            assert_eq!(call.decl_id, 0);
        }
        _ => panic!("not a call"),
    }
}

#[test]
pub fn parse_call_missing_flag_arg() {
    let parser_state = ParserState::new();
    let mut working_set = ParserWorkingSet::new(&parser_state);

    let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
    working_set.add_decl(sig.into());

    let (_, err) = working_set.parse_source(b"foo --jazz", true);
    assert!(matches!(err, Some(ParseError::MissingFlagParam(..))));
}

#[test]
pub fn parse_call_missing_short_flag_arg() {
    let parser_state = ParserState::new();
    let mut working_set = ParserWorkingSet::new(&parser_state);

    let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
    working_set.add_decl(sig.into());

    let (_, err) = working_set.parse_source(b"foo -j", true);
    assert!(matches!(err, Some(ParseError::MissingFlagParam(..))));
}

#[test]
pub fn parse_call_too_many_shortflag_args() {
    let parser_state = ParserState::new();
    let mut working_set = ParserWorkingSet::new(&parser_state);

    let sig = Signature::build("foo")
        .named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'))
        .named("--math", SyntaxShape::Int, "math!!", Some('m'));
    working_set.add_decl(sig.into());
    let (_, err) = working_set.parse_source(b"foo -mj", true);
    assert!(matches!(
        err,
        Some(ParseError::ShortFlagBatchCantTakeArg(..))
    ));
}

#[test]
pub fn parse_call_unknown_shorthand() {
    let parser_state = ParserState::new();
    let mut working_set = ParserWorkingSet::new(&parser_state);

    let sig = Signature::build("foo").switch("--jazz", "jazz!!", Some('j'));
    working_set.add_decl(sig.into());
    let (_, err) = working_set.parse_source(b"foo -mj", true);
    assert!(matches!(err, Some(ParseError::UnknownFlag(..))));
}

#[test]
pub fn parse_call_extra_positional() {
    let parser_state = ParserState::new();
    let mut working_set = ParserWorkingSet::new(&parser_state);

    let sig = Signature::build("foo").switch("--jazz", "jazz!!", Some('j'));
    working_set.add_decl(sig.into());
    let (_, err) = working_set.parse_source(b"foo -j 100", true);
    assert!(matches!(err, Some(ParseError::ExtraPositional(..))));
}

#[test]
pub fn parse_call_missing_req_positional() {
    let parser_state = ParserState::new();
    let mut working_set = ParserWorkingSet::new(&parser_state);

    let sig = Signature::build("foo").required("jazz", SyntaxShape::Int, "jazz!!");
    working_set.add_decl(sig.into());
    let (_, err) = working_set.parse_source(b"foo", true);
    assert!(matches!(err, Some(ParseError::MissingPositional(..))));
}

#[test]
pub fn parse_call_missing_req_flag() {
    let parser_state = ParserState::new();
    let mut working_set = ParserWorkingSet::new(&parser_state);

    let sig = Signature::build("foo").required_named("--jazz", SyntaxShape::Int, "jazz!!", None);
    working_set.add_decl(sig.into());
    let (_, err) = working_set.parse_source(b"foo", true);
    assert!(matches!(err, Some(ParseError::MissingRequiredFlag(..))));
}
