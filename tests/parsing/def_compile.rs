//! A `def` body is compiled to IR once, by `parse_def` after the body's scope is closed.

use nu_parser::parse;
use nu_protocol::{CompileError, engine::StateWorkingSet};
use nu_test_support::prelude::*;

#[test]
fn def_body_gets_ir() -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    parse(&mut working_set, None, b"def foo [] { 1 + 1 }", false);
    assert!(working_set.parse_errors.is_empty());
    assert!(working_set.compile_errors.is_empty());

    let decl_id = working_set.find_decl(b"foo").expect("foo is defined");
    let block_id = working_set
        .get_decl(decl_id)
        .block_id()
        .expect("a custom command has a block");
    assert!(working_set.get_block(block_id).ir_block.is_some());
    Ok(())
}

#[test]
fn failing_def_body_reports_its_compile_error_once() -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    parse(&mut working_set, None, b"def foo [] { break }", false);
    assert!(working_set.parse_errors.is_empty());
    assert!(
        matches!(
            working_set.compile_errors.as_slice(),
            [CompileError::NotInALoop { .. }]
        ),
        "{:?}",
        working_set.compile_errors
    );
    Ok(())
}
