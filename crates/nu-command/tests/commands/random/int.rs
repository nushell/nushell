use nu_test_support::prelude::*;

#[test]
fn generates_an_integer() -> Result {
    let outcome: i64 = test().run("random int 42..43")?;
    assert!(outcome == 42 || outcome == 43);
    Ok(())
}

#[test]
fn generates_55() -> Result {
    let outcome: i64 = test().run("random int 55..55")?;
    assert_eq!(outcome, 55);
    Ok(())
}

#[test]
fn generates_0() -> Result {
    let outcome: i64 = test().run("random int ..<1")?;
    assert_eq!(outcome, 0);
    Ok(())
}
