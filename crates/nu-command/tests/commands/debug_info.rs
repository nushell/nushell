use nu_test_support::prelude::*;

#[test]
fn runs_successfully() -> Result {
    let _: Value = test().run("debug info")?;
    Ok(())
}
