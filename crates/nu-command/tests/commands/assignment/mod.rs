mod concat;

use nu_test_support::nu;

#[test]
fn assign_table_cell() {
    // https://github.com/nushell/nushell/issues/17694
    let actual = nu!(r#"
        mut a = [[foo]; [bar]];
        $a.foo.0 = 'baz';
        $a.0.foo
    "#);

    assert_eq!(actual.out, "baz")
}
