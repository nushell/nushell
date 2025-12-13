use nu_test_support::nu;

#[test]
fn timeout_completes_before_limit() {
    let actual = nu!(r#"timeout 1sec { 'done' }"#);
    assert_eq!(actual.out, "done");
}

#[test]
fn timeout_with_sleep_completes_before_limit() {
    let actual = nu!(r#"timeout 500ms { sleep 100ms; 'completed' }"#);
    assert_eq!(actual.out, "completed");
}

#[test]
fn timeout_exceeds_limit() {
    let actual = nu!(r#"timeout 100ms { sleep 1sec }"#);
    assert!(actual.err.contains("timeout"));
    assert!(actual.err.contains("100ms"));
}

#[test]
fn timeout_returns_closure_result() {
    let actual = nu!(r#"timeout 1sec { 42 }"#);
    assert_eq!(actual.out, "42");
}

#[test]
fn timeout_returns_complex_value() {
    let actual = nu!(r#"timeout 1sec { {a: 1, b: 2} } | to nuon"#);
    assert_eq!(actual.out, "{a: 1, b: 2}");
}

#[test]
fn timeout_zero_duration_still_runs() {
    // Even with 0ms timeout, quick operations should complete
    let actual = nu!(r#"timeout 0ms { 'instant' }"#);
    // This may or may not complete depending on timing, but should not panic
    // If it times out, that's also acceptable behavior
    assert!(actual.out == "instant" || actual.err.contains("timeout"));
}

#[test]
fn timeout_with_computation() {
    let actual = nu!(r#"timeout 1sec { 1 + 2 + 3 }"#);
    assert_eq!(actual.out, "6");
}

#[test]
fn timeout_error_message_includes_duration() {
    let actual = nu!(r#"timeout 250ms { sleep 1sec }"#);
    assert!(actual.err.contains("250ms"));
}

#[test]
fn timeout_with_list_operations() {
    let actual = nu!(r#"timeout 1sec { [1 2 3] | each { $in * 2 } } | to nuon"#);
    assert_eq!(actual.out, "[2, 4, 6]");
}

#[test]
fn timeout_with_pipeline_input() {
    let actual = nu!(r#"[1 2 3] | timeout 1sec { $in | length }"#);
    assert_eq!(actual.out, "3");
}

#[test]
fn timeout_with_pipeline_input_record() {
    let actual = nu!(r#"{a: 1, b: 2} | timeout 1sec { $in | get a }"#);
    assert_eq!(actual.out, "1");
}
