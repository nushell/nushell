use nu_test_support::nu;

#[test]
fn while_sum() {
    let actual = nu!(
        cwd: ".",
        "mut total = 0; mut x = 0; while $x <= 10 { $total = $total + $x; $x = $x + 1 }; $total"
    );

    assert_eq!(actual.out, "55");
}

#[test]
fn while_auto_print_in_each_iteration() {
    let actual = nu!(
        cwd: ".",
        "mut total = 0; while $total < 2 { $total = $total + 1; echo 1 }"
    );
    // Note: nu! macro auto repalce "\n" and "\r\n" with ""
    // so our output will be `11`
    // that's ok, our main concern is it auto print value in each iteration.
    assert_eq!(actual.out, "11");
}

#[test]
fn while_break_on_external_failed() {
    let actual = nu!(
        cwd: ".",
        "mut total = 0; while $total < 2 { $total = $total + 1; echo 1; nu --testbin fail }"
    );
    // Note: nu! macro auto repalce "\n" and "\r\n" with ""
    // so our output will be `1`
    assert_eq!(actual.out, "1");
}
