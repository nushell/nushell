use nu_test_support::nu;

#[test]
fn test_kill_invalid_pid() {
    let pid = i32::MAX;
    let actual = nu!(format!("kill {}", pid));

    assert!(actual.err.contains("process didn't terminate successfully"));
}
