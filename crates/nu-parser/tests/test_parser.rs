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

    let (block, err) = parse(&mut working_set, None, b"3", true);

    assert!(err.is_none());
    assert!(block.len() == 1);
    match &block[0] {
        Statement::Pipeline(Pipeline { expressions }) => {
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

    let (block, err) = parse(&mut working_set, None, b"foo", true);

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

    let (_, err) = parse(&mut working_set, None, b"foo --jazz", true);
    assert!(matches!(err, Some(ParseError::MissingFlagParam(..))));
}

#[test]
pub fn parse_call_missing_short_flag_arg() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());

    let (_, err) = parse(&mut working_set, None, b"foo -j", true);
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
    let (_, err) = parse(&mut working_set, None, b"foo -mj", true);
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
    let (_, err) = parse(&mut working_set, None, b"foo -mj", true);
    assert!(matches!(err, Some(ParseError::UnknownFlag(..))));
}

#[test]
pub fn parse_call_extra_positional() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").switch("--jazz", "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());
    let (_, err) = parse(&mut working_set, None, b"foo -j 100", true);
    assert!(matches!(err, Some(ParseError::ExtraPositional(..))));
}

#[test]
pub fn parse_call_missing_req_positional() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").required("jazz", SyntaxShape::Int, "jazz!!");
    working_set.add_decl(sig.predeclare());
    let (_, err) = parse(&mut working_set, None, b"foo", true);
    assert!(matches!(err, Some(ParseError::MissingPositional(..))));
}

#[test]
pub fn parse_call_missing_req_flag() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").required_named("--jazz", SyntaxShape::Int, "jazz!!", None);
    working_set.add_decl(sig.predeclare());
    let (_, err) = parse(&mut working_set, None, b"foo", true);
    assert!(matches!(err, Some(ParseError::MissingRequiredFlag(..))));
}

mod range {
    use super::*;
    use nu_protocol::ast::{RangeInclusion, RangeOperator};

    #[test]
    fn parse_inclusive_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"0..10", true);

        assert!(err.is_none());
        assert!(block.len() == 1);
        match &block[0] {
            Statement::Pipeline(Pipeline { expressions }) => {
                assert!(expressions.len() == 1);
                assert!(matches!(
                    expressions[0],
                    Expression {
                        expr: Expr::Range(
                            Some(_),
                            Some(_),
                            RangeOperator {
                                inclusion: RangeInclusion::Inclusive,
                                ..
                            }
                        ),
                        ..
                    }
                ))
            }
            _ => panic!("No match"),
        }
    }

    #[test]
    fn parse_exclusive_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"0..<10", true);

        assert!(err.is_none());
        assert!(block.len() == 1);
        match &block[0] {
            Statement::Pipeline(Pipeline { expressions }) => {
                assert!(expressions.len() == 1);
                assert!(matches!(
                    expressions[0],
                    Expression {
                        expr: Expr::Range(
                            Some(_),
                            Some(_),
                            RangeOperator {
                                inclusion: RangeInclusion::RightExclusive,
                                ..
                            }
                        ),
                        ..
                    }
                ))
            }
            _ => panic!("No match"),
        }
    }

    #[test]
    fn parse_subexpression_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"(3 - 3)..<(8 + 2)", true);

        assert!(err.is_none());
        assert!(block.len() == 1);
        match &block[0] {
            Statement::Pipeline(Pipeline { expressions }) => {
                assert!(expressions.len() == 1);
                assert!(matches!(
                    expressions[0],
                    Expression {
                        expr: Expr::Range(
                            Some(_),
                            Some(_),
                            RangeOperator {
                                inclusion: RangeInclusion::RightExclusive,
                                ..
                            }
                        ),
                        ..
                    }
                ))
            }
            _ => panic!("No match"),
        }
    }

    #[test]
    fn parse_variable_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"let a = 2; $a..10", true);

        assert!(err.is_none());
        assert!(block.len() == 2);
        match &block[1] {
            Statement::Pipeline(Pipeline { expressions }) => {
                assert!(expressions.len() == 1);
                assert!(matches!(
                    expressions[0],
                    Expression {
                        expr: Expr::Range(
                            Some(_),
                            Some(_),
                            RangeOperator {
                                inclusion: RangeInclusion::Inclusive,
                                ..
                            }
                        ),
                        ..
                    }
                ))
            }
            _ => panic!("No match"),
        }
    }

    #[test]
    fn parse_subexpression_variable_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"let a = 2; $a..<($a + 10)", true);

        assert!(err.is_none());
        assert!(block.len() == 2);
        match &block[1] {
            Statement::Pipeline(Pipeline { expressions }) => {
                assert!(expressions.len() == 1);
                assert!(matches!(
                    expressions[0],
                    Expression {
                        expr: Expr::Range(
                            Some(_),
                            Some(_),
                            RangeOperator {
                                inclusion: RangeInclusion::RightExclusive,
                                ..
                            }
                        ),
                        ..
                    }
                ))
            }
            _ => panic!("No match"),
        }
    }

    #[test]
    fn parse_right_unbounded_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"0..", true);

        assert!(err.is_none());
        assert!(block.len() == 1);
        match &block[0] {
            Statement::Pipeline(Pipeline { expressions }) => {
                assert!(expressions.len() == 1);
                assert!(matches!(
                    expressions[0],
                    Expression {
                        expr: Expr::Range(
                            Some(_),
                            None,
                            RangeOperator {
                                inclusion: RangeInclusion::Inclusive,
                                ..
                            }
                        ),
                        ..
                    }
                ))
            }
            _ => panic!("No match"),
        }
    }

    #[test]
    fn parse_negative_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"-10..-3", true);

        assert!(err.is_none());
        assert!(block.len() == 1);
        match &block[0] {
            Statement::Pipeline(Pipeline { expressions }) => {
                assert!(expressions.len() == 1);
                assert!(matches!(
                    expressions[0],
                    Expression {
                        expr: Expr::Range(
                            Some(_),
                            Some(_),
                            RangeOperator {
                                inclusion: RangeInclusion::Inclusive,
                                ..
                            }
                        ),
                        ..
                    }
                ))
            }
            _ => panic!("No match"),
        }
    }

    #[test]
    fn bad_parse_does_crash() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (_, err) = parse(&mut working_set, None, b"(0)..\"a\"", true);

        assert!(err.is_some());
    }
}
