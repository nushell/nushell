mod concat;

use nu_experimental::REORDER_CELL_PATHS;
use nu_test_support::prelude::*;

#[test]
#[exp(REORDER_CELL_PATHS)]
fn assign_table_cell() -> Result {
    // ensure the experimental option is enabled for the regression case
    let code = "
        mut a = [[foo]; [bar]];
        $a.foo.0 = 'baz';
        $a.0.foo
    ";

    test().run(code).expect_value_eq("baz")
}

#[test]
#[exp(REORDER_CELL_PATHS)]
fn assign_table_cell_multiple_ints() -> Result {
    // path with more than one integer should still work when reordering
    let code = "
        mut a = [ [[foo]; [bar]] ];
        $a.0.0.foo = 'hi';
        $a.0.0.foo
    ";

    test().run(code).expect_value_eq("hi")
}

#[test]
#[exp(REORDER_CELL_PATHS)]
fn assign_table_cell_mixed_rows() -> Result {
    // regression: table with header then string rows should allow column access
    let code = "
        mut table = [ [foo]; ['a'] ['b'] ];
        $table.foo.0 = 'z';
        $table.foo.0
    ";

    test().run(code).expect_value_eq("z")
}
