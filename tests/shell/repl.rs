use nu_test_support::prelude::*;

#[test]
fn mut_variable() -> Result {
    let mut tester = test();
    let () = tester.run("mut x = 0")?;
    let () = tester.run("$x = 1")?;
    tester.run("$x").expect_value_eq(1)
}
