use nu_test_support::prelude::*;

#[test]
fn columns_propagates_error_values_in_stream() -> Result {
    test()
        .run("[[name size]; [a 100b] [b 200b]] | where size <= 150 | columns")
        .expect_error_code_eq("nu::shell::operator_incompatible_types")
}
