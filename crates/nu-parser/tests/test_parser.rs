use nu_parser::ParseError;
use nu_parser::*;
use nu_protocol::{
    ast::{Expr, Expression},
    engine::{Command, EngineState, Stack, StateWorkingSet},
    Signature, SyntaxShape,
};

#[cfg(test)]
#[derive(Clone)]
pub struct Let;

#[cfg(test)]
impl Command for Let {
    fn name(&self) -> &str {
        "let"
    }

    fn usage(&self) -> &str {
        "Create a variable and give it a value."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("let")
            .required("var_name", SyntaxShape::VarWithOptType, "variable name")
            .required(
                "initial_value",
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::Expression)),
                "equals sign followed by value",
            )
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &nu_protocol::ast::Call,
        _input: nu_protocol::PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        todo!()
    }
}

#[test]
pub fn parse_int() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(&mut working_set, None, b"3", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);
    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert!(matches!(
        expressions[0],
        Expression {
            expr: Expr::Int(3),
            ..
        }
    ))
}

#[test]
pub fn parse_binary_with_hex_format() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(&mut working_set, None, b"0x[13]", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);
    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert_eq!(expressions[0].expr, Expr::Binary(vec![0x13]))
}

#[test]
pub fn parse_binary_with_incomplete_hex_format() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(&mut working_set, None, b"0x[3]", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);
    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert_eq!(expressions[0].expr, Expr::Binary(vec![0x03]))
}

#[test]
pub fn parse_binary_with_binary_format() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(&mut working_set, None, b"0b[1010 1000]", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);
    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert_eq!(expressions[0].expr, Expr::Binary(vec![0b10101000]))
}

#[test]
pub fn parse_binary_with_incomplete_binary_format() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(&mut working_set, None, b"0b[10]", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);
    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert_eq!(expressions[0].expr, Expr::Binary(vec![0b00000010]))
}

#[test]
pub fn parse_binary_with_octal_format() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(&mut working_set, None, b"0o[250]", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);
    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert_eq!(expressions[0].expr, Expr::Binary(vec![0o250]))
}

#[test]
pub fn parse_binary_with_incomplete_octal_format() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(&mut working_set, None, b"0o[2]", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);
    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert_eq!(expressions[0].expr, Expr::Binary(vec![0o2]))
}

#[test]
pub fn parse_binary_with_invalid_octal_format() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(&mut working_set, None, b"0b[90]", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);
    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert!(match &expressions[0].expr {
        Expr::Binary(_) => false,
        _ => true,
    })
}

#[test]
pub fn parse_string() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(&mut working_set, None, b"\"hello nushell\"", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);
    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert_eq!(
        expressions[0].expr,
        Expr::String("hello nushell".to_string())
    )
}

#[test]
pub fn parse_escaped_string() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(
        &mut working_set,
        None,
        b"\"hello \\u006e\\u0075\\u0073hell\"",
        true,
        &[],
    );

    assert!(err.is_none());
    assert!(block.len() == 1);
    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert_eq!(
        expressions[0].expr,
        Expr::String("hello nushell".to_string())
    )
}

#[test]
pub fn parse_call() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());

    let (block, err) = parse(&mut working_set, None, b"foo", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);

    let expressions = &block[0];
    assert_eq!(expressions.len(), 1);

    if let Expression {
        expr: Expr::Call(call),
        ..
    } = &expressions[0]
    {
        assert_eq!(call.decl_id, 0);
    }
}

#[test]
pub fn parse_call_missing_flag_arg() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").named("jazz", SyntaxShape::Int, "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());

    let (_, err) = parse(&mut working_set, None, b"foo --jazz", true, &[]);
    assert!(matches!(err, Some(ParseError::MissingFlagParam(..))));
}

#[test]
pub fn parse_call_missing_short_flag_arg() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());

    let (_, err) = parse(&mut working_set, None, b"foo -j", true, &[]);
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
    let (_, err) = parse(&mut working_set, None, b"foo -mj", true, &[]);
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
    let (_, err) = parse(&mut working_set, None, b"foo -mj", true, &[]);
    assert!(matches!(err, Some(ParseError::UnknownFlag(..))));
}

#[test]
pub fn parse_call_extra_positional() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").switch("--jazz", "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());
    let (_, err) = parse(&mut working_set, None, b"foo -j 100", true, &[]);
    assert!(matches!(err, Some(ParseError::ExtraPositional(..))));
}

#[test]
pub fn parse_call_missing_req_positional() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").required("jazz", SyntaxShape::Int, "jazz!!");
    working_set.add_decl(sig.predeclare());
    let (_, err) = parse(&mut working_set, None, b"foo", true, &[]);
    assert!(matches!(err, Some(ParseError::MissingPositional(..))));
}

#[test]
pub fn parse_call_missing_req_flag() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").required_named("--jazz", SyntaxShape::Int, "jazz!!", None);
    working_set.add_decl(sig.predeclare());
    let (_, err) = parse(&mut working_set, None, b"foo", true, &[]);
    assert!(matches!(err, Some(ParseError::MissingRequiredFlag(..))));
}

#[test]
fn test_nothing_comparisson_eq() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);
    let (block, err) = parse(&mut working_set, None, b"2 == $nothing", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);

    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert!(matches!(
        &expressions[0],
        Expression {
            expr: Expr::BinaryOp(..),
            ..
        }
    ))
}

#[test]
fn test_nothing_comparisson_neq() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);
    let (block, err) = parse(&mut working_set, None, b"2 != $nothing", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);

    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert!(matches!(
        &expressions[0],
        Expression {
            expr: Expr::BinaryOp(..),
            ..
        }
    ))
}

mod range {
    use super::*;
    use nu_protocol::ast::{RangeInclusion, RangeOperator};

    #[test]
    fn parse_inclusive_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"0..10", true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 1);

        let expressions = &block[0];
        assert!(expressions.len() == 1);
        assert!(matches!(
            expressions[0],
            Expression {
                expr: Expr::Range(
                    Some(_),
                    None,
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

    #[test]
    fn parse_exclusive_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"0..<10", true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 1);

        let expressions = &block[0];
        assert!(expressions.len() == 1);
        assert!(matches!(
            expressions[0],
            Expression {
                expr: Expr::Range(
                    Some(_),
                    None,
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

    #[test]
    fn parse_reverse_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"10..0", true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 1);

        let expressions = &block[0];
        assert!(expressions.len() == 1);
        assert!(matches!(
            expressions[0],
            Expression {
                expr: Expr::Range(
                    Some(_),
                    None,
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

    #[test]
    fn parse_subexpression_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"(3 - 3)..<(8 + 2)", true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 1);

        let expressions = &block[0];
        assert!(expressions.len() == 1);
        assert!(matches!(
            expressions[0],
            Expression {
                expr: Expr::Range(
                    Some(_),
                    None,
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

    #[test]
    fn parse_variable_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        working_set.add_decl(Box::new(Let));

        let (block, err) = parse(&mut working_set, None, b"let a = 2; $a..10", true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 2);

        let expressions = &block[1];
        assert!(expressions.len() == 1);
        assert!(matches!(
            expressions[0],
            Expression {
                expr: Expr::Range(
                    Some(_),
                    None,
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

    #[test]
    fn parse_subexpression_variable_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        working_set.add_decl(Box::new(Let));

        let (block, err) = parse(
            &mut working_set,
            None,
            b"let a = 2; $a..<($a + 10)",
            true,
            &[],
        );

        assert!(err.is_none());
        assert!(block.len() == 2);

        let expressions = &block[1];
        assert!(expressions.len() == 1);
        assert!(matches!(
            expressions[0],
            Expression {
                expr: Expr::Range(
                    Some(_),
                    None,
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

    #[test]
    fn parse_right_unbounded_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"0..", true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 1);

        let expressions = &block[0];
        assert!(expressions.len() == 1);
        assert!(matches!(
            expressions[0],
            Expression {
                expr: Expr::Range(
                    Some(_),
                    None,
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

    #[test]
    fn parse_left_unbounded_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"..10", true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 1);

        let expressions = &block[0];
        assert!(expressions.len() == 1);
        assert!(matches!(
            expressions[0],
            Expression {
                expr: Expr::Range(
                    None,
                    None,
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

    #[test]
    fn parse_negative_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"-10..-3", true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 1);

        let expressions = &block[0];
        assert!(expressions.len() == 1);
        assert!(matches!(
            expressions[0],
            Expression {
                expr: Expr::Range(
                    Some(_),
                    None,
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

    #[test]
    fn parse_float_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"2.0..4.0..10.0", true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 1);

        let expressions = &block[0];
        assert!(expressions.len() == 1);
        assert!(matches!(
            expressions[0],
            Expression {
                expr: Expr::Range(
                    Some(_),
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

    #[test]
    fn bad_parse_does_crash() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (_, err) = parse(&mut working_set, None, b"(0)..\"a\"", true, &[]);

        assert!(err.is_some());
    }
}
