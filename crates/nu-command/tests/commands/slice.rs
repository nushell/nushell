use nu_test_support::prelude::*;
use rstest::rstest;

#[rstest]
#[case::one_row("slice 0..0", [0])]
#[case::some_rows("slice 1..2", [1, 2])]
#[case::negative_indices("slice (-1..)", [3])]
#[case::zero_to_zero_exclusive("slice 0..<0", [(); 0])]
#[case::to_negative_one_inclusive("slice 2..-1", [2, 3])]
fn test_slice(#[case] code: &str, #[case] expect: impl IntoValue) -> Result {
    test()
        .run_with_data(code, [0, 1, 2, 3])
        .expect_value_eq(expect)
}
