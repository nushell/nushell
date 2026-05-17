use nu_test_support::{nu, nu_repl_code};
use pretty_assertions::assert_eq;

#[test]
fn mut_variable() {
    let lines = &["mut x = 0", "$x = 1", "$x"];
    let actual = nu!(nu_repl_code(lines));
    assert_eq!(actual.out, "1");
}
