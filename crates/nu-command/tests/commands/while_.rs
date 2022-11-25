use nu_test_support::nu;

#[test]
fn while_sum() {
    let actual = nu!(
        cwd: ".",
        "mut total = 0; mut x = 0; while $x <= 10 { $total = $total + $x; $x = $x + 1 }; $total"
    );

    assert_eq!(actual.out, "55");
}
