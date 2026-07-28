use nu_test_support::prelude::*;

#[test]
fn not_empty() -> Result {
    test()
        .run("keybindings list | is-not-empty")
        .expect_value_eq(true)
}
