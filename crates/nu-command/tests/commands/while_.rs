use nu_test_support::nu;

#[test]
fn while_sum() {
    let actual = nu!(
        "mut total = 0; mut x = 0; while $x <= 10 { $total = $total + $x; $x = $x + 1 }; $total"
    );

    assert_eq!(actual.out, "55");
}

#[test]
fn while_doesnt_auto_print_in_each_iteration() {
    let actual = nu!("mut total = 0; while $total < 2 { $total = $total + 1; 1 }");
    // Make sure we don't see any of these values in the output
    // As we do not auto-print loops anymore
    assert!(!actual.out.contains('1'));
}

#[test]
fn while_break_on_external_failed() {
    let actual =
        nu!("mut total = 0; while $total < 2 { $total = $total + 1; print 1; nu --testbin fail }");
    // Note: nu! macro auto replace "\n" and "\r\n" with ""
    // so our output will be `1`
    assert_eq!(actual.out, "1");
}

#[test]
fn failed_while_should_break_running() {
    let actual =
        nu!("mut total = 0; while $total < 2 { $total = $total + 1; nu --testbin fail }; print 3");
    assert!(!actual.out.contains('3'));
}
