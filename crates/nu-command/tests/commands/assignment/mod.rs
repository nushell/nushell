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

/// Field assignment on a mutable record (IR `update-var-cell-path` path).
#[test]
fn mut_record_field_assign() -> Result {
    let code = "
        mut r = {a: 1, b: 2}
        $r.a = 9
        $r == {a: 9, b: 2}
    ";

    test().run(code).expect_value_eq(true)
}

/// List element assignment on a mutable list.
#[test]
fn mut_list_index_assign() -> Result {
    let code = "
        mut l = [10 20 30]
        $l.1 = 99
        $l
    ";

    test().run(code).expect_value_eq([10, 99, 30])
}

/// Nested cell path assignment.
#[test]
fn mut_nested_field_assign() -> Result {
    let code = "
        mut r = {outer: {inner: 1}}
        $r.outer.inner = 42
        $r.outer.inner
    ";

    test().run(code).expect_value_eq(42)
}

/// Compound assignment still ends with in-place field update.
#[test]
fn mut_record_field_compound_assign() -> Result {
    let code = "
        mut r = {a: 5}
        $r.a += 3
        $r.a
    ";

    test().run(code).expect_value_eq(8)
}

/// Many field writes on a large payload (correctness of the hot path).
#[test]
fn mut_record_field_assign_many_times() -> Result {
    let code = "
        mut r = {a: (1..500 | each {|i| $i | into string} | str join)}
        for _ in 1..200 { $r.a = 'x' }
        $r.a
    ";

    test().run(code).expect_value_eq("x")
}

/// Compiler must emit `update-var-cell-path` for `$r.field = …` (not load+upsert+store).
#[test]
fn mut_field_assign_compiles_to_update_var_cell_path() -> Result {
    let code = "
        def __mut_field_assign_ir [] {
            mut r = {a: 1}
            $r.a = 2
            $r
        }
        view ir __mut_field_assign_ir
    ";

    let ir: String = test().run(code)?;
    assert!(
        ir.contains("update-var-cell-path"),
        "expected UpdateVarCellPath in IR, got:\n{ir}"
    );
    // The optimized path must not fall back to UpsertCellPath + StoreVariable on the var.
    assert!(
        !ir.contains("upsert-cell-path"),
        "expected no UpsertCellPath for simple mut field assign, got:\n{ir}"
    );
    Ok(())
}
