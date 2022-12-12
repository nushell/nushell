use nu_test_support::nu;

#[test]
fn loop_auto_print_in_each_iteration() {
    let actual = nu!(
        cwd: ".",
        r#"
        mut total = 0;
        loop {
            if $total == 3 {
                break;
            } else {
                $total += 1;
            }
            echo 1
        }"#
    );
    // Note: nu! macro auto repalce "\n" and "\r\n" with ""
    // so our output will be `111`
    // that's ok, our main concern is it auto print value in each iteration.
    assert_eq!(actual.out, "111");
}
