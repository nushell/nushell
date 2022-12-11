use nu_test_support::nu;

#[test]
fn for_auto_print_in_each_iteration() {
    let actual = nu!(
        cwd: ".",
        r#"
        for i in 1..2 {
            echo 1
        }"#
    );
    // Note: nu! macro auto repalce "\n" and "\r\n" with ""
    // so our output will be `11`
    // that's ok, our main concern is it auto print value in each iteration.
    assert_eq!(actual.out, "11");
}
