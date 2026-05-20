use nu_test_support::prelude::*;

#[test]
fn test_uname_all() -> Result {
    test().run("uname").map(|_: Value| ())
}
