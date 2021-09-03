use nu_parser::ParseError;
use nu_parser::*;
use nu_protocol::{
    ast::{Expr, Expression, Pipeline, Statement},
    engine::{EngineState, StateWorkingSet},
    Signature, SyntaxShape,
};

#[test]
pub fn parse_int() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse_source(&mut working_set, b"3", true);

    assert!(err.is_none());
    assert!(block.len() == 1);
    match &block[0] {
        Statement::Pipeline(Pipeline {
            expressions: expressions,
        }) => {
            assert!(expressions.len() == 1);
            assert!(matches!(
                expressions[0],
                Expression {
                    expr: Expr::Int(3),
                    ..
                }
            ))
        }
        _ => panic!("No match"),
    }
}

#[test]
pub fn parse_call() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());

    let (block, err) = parse_source(&mut working_set, b"foo", true);

    assert!(err.is_none());
    assert!(block.len() == 1);

    match &block[0] {
        Statement::Pipeline(Pipeline { expressions }) => {
            assert_eq!(expressions.len(), 1);

            if let Expression {
                expr: Expr::Call(call),
                ..
            } = &expressions[0]
            {
                assert_eq!(call.decl_id, 0);
            }
        }
        _ => panic!("not a call"),
    }
}

#[test]
pub fn parse_call_missing_flag_arg() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());

    let (_, err) = parse_source(&mut working_set, b"foo --jazz", true);
    assert!(matches!(err, Some(ParseError::MissingFlagParam(..))));
}

#[test]
pub fn parse_call_missing_short_flag_arg() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());

    let (_, err) = parse_source(&mut working_set, b"foo -j", true);
    assert!(matches!(err, Some(ParseError::MissingFlagParam(..))));
}

#[test]
pub fn parse_call_too_many_shortflag_args() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo")
        .named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'))
        .named("--math", SyntaxShape::Int, "math!!", Some('m'));
    working_set.add_decl(sig.predeclare());
    let (_, err) = parse_source(&mut working_set, b"foo -mj", true);
    assert!(matches!(
        err,
        Some(ParseError::ShortFlagBatchCantTakeArg(..))
    ));
}

#[test]
pub fn parse_call_unknown_shorthand() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").switch("--jazz", "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());
    let (_, err) = parse_source(&mut working_set, b"foo -mj", true);
    assert!(matches!(err, Some(ParseError::UnknownFlag(..))));
}

#[test]
pub fn parse_call_extra_positional() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").switch("--jazz", "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());
    let (_, err) = parse_source(&mut working_set, b"foo -j 100", true);
    assert!(matches!(err, Some(ParseError::ExtraPositional(..))));
}

#[test]
pub fn parse_call_missing_req_positional() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").required("jazz", SyntaxShape::Int, "jazz!!");
    working_set.add_decl(sig.predeclare());
    let (_, err) = parse_source(&mut working_set, b"foo", true);
    assert!(matches!(err, Some(ParseError::MissingPositional(..))));
}

#[test]
pub fn parse_call_missing_req_flag() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").required_named("--jazz", SyntaxShape::Int, "jazz!!", None);
    working_set.add_decl(sig.predeclare());
    let (_, err) = parse_source(&mut working_set, b"foo", true);
    assert!(matches!(err, Some(ParseError::MissingRequiredFlag(..))));
}
