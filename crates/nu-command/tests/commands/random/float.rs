use nu_test_support::prelude::*;

#[test]
fn rejects_empty_range() -> Result {
    let error = test().run("random float 1..<1").expect_shell_error()?;
    assert!(matches!(error, ShellError::InvalidRange { .. }));
    Ok(())
}

#[test]
fn rejects_unrepresentable_range() -> Result {
    for range in ["(-1e308)..1e308", "(-1e308)..<1e308", "(-1e308).."] {
        let error = test()
            .run(format!("random float {range}"))
            .expect_shell_error()?;
        assert!(matches!(error, ShellError::InvalidRange { .. }));
    }
    Ok(())
}

#[test]
fn generates_negative_float() -> Result {
    let outcome: f64 = test().run("random float (-2.0)..(-1.0)")?;
    assert!((-2.0..=-1.0).contains(&outcome));
    Ok(())
}

#[test]
fn generates_a_float() -> Result {
    let outcome: f64 = test().run("random float 42..43")?;
    assert!((42.0..=43.0).contains(&outcome));

    test()
        .run("random float 42..43 | describe")
        .expect_value_eq("float")?;
    Ok(())
}

#[test]
fn generates_55() -> Result {
    test().run("random float 55..55").expect_value_eq(55.0)
}

#[test]
fn generates_0() -> Result {
    let outcome: f64 = test().run("random float ..<1")?;
    assert!((0.0..1.0).contains(&outcome));
    Ok(())
}

#[test]
fn generate_inf() -> Result {
    test()
        .run("random float 1.. | describe")
        .expect_value_eq("float")
}
