mod concat;

use nu_test_support::nu;

#[test]
fn assign_table_cell() {
    // ensure the experimental option is enabled for the regression case
    let actual = nu!(experimental: vec!["reorder-cell-paths".to_string()], r#"
        mut a = [[foo]; [bar]];
        $a.foo.0 = 'baz';
        $a.0.foo
    "#);

    assert_eq!(actual.out, "baz")
}

#[test]
fn assign_table_cell_multiple_ints() {
    // path with more than one integer should still work when reordering
    let actual = nu!(experimental: vec!["reorder-cell-paths".to_string()], r#"
        mut a = [ [[foo]; [bar]] ];
        $a.0.0.foo = 'hi';
        $a.0.0.foo
    "#);

    assert_eq!(actual.out, "hi")
}

#[test]
fn assign_table_cell_mixed_rows() {
    // regression: table with header then string rows should allow column access
    let actual = nu!(experimental: vec!["reorder-cell-paths".to_string()], r#"
        mut table = [ [foo]; ['a'] ['b'] ];
        $table.foo.0 = 'z';
        $table.foo.0
    "#);

    assert_eq!(actual.out, "z")
}
