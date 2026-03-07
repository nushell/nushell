use nu_test_support::prelude::*;

#[test]
fn generates_a_float() -> Result {
    let outcome: f64 = test().run("random float 42..43")?;
    assert!((42.0..=43.0).contains(&outcome));

    let outcome: String = test().run("random float 42..43 | describe")?;
    assert_eq!(outcome, "float");
    Ok(())
}

#[test]
fn generates_55() -> Result {
    let outcome: f64 = test().run("random float 55..55")?;
    assert_eq!(outcome, 55.0);
    Ok(())
}

#[test]
fn generates_0() -> Result {
    let outcome: f64 = test().run("random float ..<1")?;
    assert!((0.0..1.0).contains(&outcome));
    Ok(())
}

#[test]
fn generate_inf() -> Result {
    let outcome: String = test().run("random float 1.. | describe")?;
    assert_eq!(outcome, "float");
    Ok(())
}
