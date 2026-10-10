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

/// The compile errors of parsing `source`, which must parse without errors.
fn compile_errors(source: &str) -> Vec<CompileError> {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    parse(&mut working_set, None, source.as_bytes(), false);
    assert!(
        working_set.parse_errors.is_empty(),
        "{source}: {:?}",
        working_set.parse_errors
    );
    working_set.compile_errors
}

#[test]
fn failing_def_body_reports_its_compile_error_once() -> Result {
    let errors = compile_errors("def foo [] { break }");
    assert!(
        matches!(errors.as_slice(), [CompileError::NotInALoop { .. }]),
        "{errors:?}"
    );
    Ok(())
}

/// `try` discards the `NotInALoop` error of a `catch` or `finally` closure that uses `break` or
/// `continue` itself (it runs inline, so inside the enclosing loop), but not the error of a `def`
/// body or closure nested in it, which is compiled on its own.
#[test]
fn try_clause_keeps_the_errors_of_nested_bodies() -> Result {
    for source in [
        "try { } catch { def foo [] { break } }",
        "try { } finally { def foo [] { continue } }",
        "try { } catch {|e| do { break } }",
    ] {
        let errors = compile_errors(source);
        assert!(
            matches!(errors.as_slice(), [CompileError::NotInALoop { .. }]),
            "{source}: {errors:?}"
        );
    }
    for source in [
        "loop { try { } catch {|e| break } }",
        "loop { try { } finally { continue } }",
    ] {
        let errors = compile_errors(source);
        assert!(errors.is_empty(), "{source}: {errors:?}");
    }
    Ok(())
}
