use nu_test_support::nu;

#[test]
fn not_empty() {
    let result = nu!("keybindings list | is-not-empty");
    assert_eq!(result.out, "true");
}
