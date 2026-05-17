use nu_test_support::prelude::*;

#[test]
fn record_map_to_toml() -> Result {
    let code = "
        {a: 1 b: 2 c: 'qwe'} 
        | to toml
        | from toml
        | $in == {a: 1 b: 2 c: 'qwe'}
    ";

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn nested_records_to_toml() -> Result {
    let code = "
        {a: {a: a b: b} c: 1} 
        | to toml
        | from toml
        | $in == {a: {a: a b: b} c: 1}
    ";

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn records_with_tables_to_toml() -> Result {
    let code = "
        {a: [[a b]; [1 2] [3 4]] b: [[c d e]; [1 2 3]]}
        | to toml
        | from toml
        | $in == {a: [[a b]; [1 2] [3 4]] b: [[c d e]; [1 2 3]]}
    ";

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn nested_tables_to_toml() -> Result {
    let code = "
        {c: [[f g]; [[[h k]; [1 2] [3 4]] 1]]}
        | to toml
        | from toml
        | $in == {c: [[f g]; [[[h k]; [1 2] [3 4]] 1]]}
    ";

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn table_to_toml_fails() -> Result {
    // Tables can't be represented in toml
    let code = "
        try { [[a b]; [1 2] [5 6]] | to toml | false } catch { true }
    ";

    let err = test().run(code).expect_parse_error()?;
    assert!(matches!(err, ParseError::InputMismatch(..)));
    Ok(())
}

#[test]
fn string_to_toml_fails() -> Result {
    // Strings are not a top-level toml structure
    let code = "
        try { 'not a valid toml' | to toml | false } catch { true }
    ";

    let err = test().run(code).expect_parse_error()?;
    assert!(matches!(err, ParseError::InputMismatch(..)));
    Ok(())
}

#[test]
fn big_record_to_toml_text_and_from_toml_text_back_into_record() -> Result {
    let code = "
        open cargo_sample.toml
        | to toml
        | from toml
        | get package.name
    ";

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("nu")
}
