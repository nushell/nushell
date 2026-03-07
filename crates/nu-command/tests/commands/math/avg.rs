use nu_test_support::prelude::*;

#[test]
fn can_average_numbers() -> Result {
    let code = r#"
        open sgml_description.json
        | get glossary.GlossDiv.GlossList.GlossEntry.Sections
        | math avg
     "#;
    let outcome: f64 = test().cwd("tests/fixtures/formats").run(code)?;

    assert_eq!(outcome, 101.5);
    Ok(())
}

#[test]
fn can_average_bytes() -> Result {
    let outcome: String = test().run("[100kb, 10b, 100mib] | math avg | to json -r")?;

    assert_eq!(outcome, "34985870");
    Ok(())
}

#[test]
fn can_average_range() -> Result {
    let outcome: f64 = test().run("0..5 | math avg")?;

    assert_eq!(outcome, 2.5);
    Ok(())
}

#[test]
fn cannot_average_infinite_range() -> Result {
    let outcome = test().run("0.. | math avg").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}

#[test]
fn const_avg() -> Result {
    let outcome: f64 = test().run("const AVG = [1 3 5] | math avg; $AVG")?;
    assert_eq!(outcome, 3.0);
    Ok(())
}
