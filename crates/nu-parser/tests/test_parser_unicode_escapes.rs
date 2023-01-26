#![cfg(test)]

use nu_parser::ParseError;
use nu_parser::*;
use nu_protocol::{
    ast::{Expr, Expression, PipelineElement},
    engine::{Command, EngineState, Stack, StateWorkingSet},
    Signature, SyntaxShape,
};

#[test]
pub fn parse_unicode_escaped_string1() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(
        &mut working_set,
        None,
        b"\"hello \\u{6e}\\u{000075}\\u{073}hell\"",
        true,
        &[],
    );

    assert!(err.is_none());
    assert_eq!(block.len(), 1);
    let expressions = &block[0];
    assert_eq!(expressions.len(), 1);
    if let PipelineElement::Expression(_, expr) = &expressions[0] {
        assert_eq!(expr.expr, Expr::String("hello nushell".to_string()))
    } else {
        panic!("Not an expression")
    }
}
