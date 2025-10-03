use nu_test_support::nu;

#[test]
fn record_map_to_toml() {
    let actual = nu!(r#"
        {a: 1 b: 2 c: 'qwe'} 
        | to toml
        | from toml
        | $in == {a: 1 b: 2 c: 'qwe'}
    "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn nested_records_to_toml() {
    let actual = nu!(r#"
        {a: {a: a b: b} c: 1} 
        | to toml
        | from toml
        | $in == {a: {a: a b: b} c: 1}
    "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn records_with_tables_to_toml() {
    let actual = nu!(r#"
        {a: [[a b]; [1 2] [3 4]] b: [[c d e]; [1 2 3]]}
        | to toml
        | from toml
        | $in == {a: [[a b]; [1 2] [3 4]] b: [[c d e]; [1 2 3]]}
    "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn nested_tables_to_toml() {
    let actual = nu!(r#"
        {c: [[f g]; [[[h k]; [1 2] [3 4]] 1]]}
        | to toml
        | from toml
        | $in == {c: [[f g]; [[[h k]; [1 2] [3 4]] 1]]}
    "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn table_to_toml_fails() {
    // Tables can't be represented in toml
    let actual = nu!(r#"
    try { [[a b]; [1 2] [5 6]] | to toml | false } catch { true }
    "#);

    assert!(actual.err.contains("command doesn't support"));
}

#[test]
fn string_to_toml_fails() {
    // Strings are not a top-level toml structure
    let actual = nu!(r#"
    try { 'not a valid toml' | to toml | false } catch { true }
    "#);

    assert!(actual.err.contains("command doesn't support"));
}

#[test]
fn big_record_to_toml_text_and_from_toml_text_back_into_record() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        open cargo_sample.toml
        | to toml
        | from toml
        | get package.name
    "#);

    assert_eq!(actual.out, "nu");
}
