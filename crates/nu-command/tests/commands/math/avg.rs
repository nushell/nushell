use nu_test_support::prelude::*;

#[test]
fn can_average_numbers() -> Result {
    let code = "
        open sgml_description.json
        | get glossary.GlossDiv.GlossList.GlossEntry.Sections
        | math avg
     ";
    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(101.5)
}

#[test]
fn can_average_bytes() -> Result {
    test()
        .run("[100kb, 10b, 100mib] | math avg | to json -r")
        .expect_value_eq("34985870")
}

#[test]
fn can_average_range() -> Result {
    test().run("0..5 | math avg").expect_value_eq(2.5)
}

#[test]
fn cannot_average_infinite_range() -> Result {
    let outcome = test().run("0.. | math avg").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}

#[test]
fn const_avg() -> Result {
    test()
        .run("const AVG = [1 3 5] | math avg; $AVG")
        .expect_value_eq(3.0)
}
