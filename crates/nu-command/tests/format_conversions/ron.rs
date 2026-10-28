use nu_test_support::{fs::Stub::FileWithContentToBeTrimmed, prelude::*};

#[test]
fn ron_roundtrip_string() -> Result {
    let code = r#"
        "hello world"
        | to ron
        | from ron
        | $in == "hello world"
    "#;

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn ron_roundtrip_integer() -> Result {
    let code = r#"
        42
        | to ron
        | from ron
        | $in == 42
    "#;

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn ron_roundtrip_float() -> Result {
    let code = r#"
        3.14
        | to ron
        | from ron
        | $in == 3.14
    "#;

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn ron_roundtrip_boolean() -> Result {
    let code = r#"
        true
        | to ron
        | from ron
        | $in == true
    "#;

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn ron_roundtrip_record() -> Result {
    let code = r#"
        {a: 1 b: 2 c: 'qwe'}
        | to ron
        | from ron
        | $in == {a: 1 b: 2 c: 'qwe'}
    "#;
    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn ron_roundtrip_map() -> Result {
    let code = r#"
        {a: 1 b: 2 c: 'qwe'}
        | into record
        | to ron
        | from ron
        | $in == {a: 1 b: 2 c: 'qwe'}
    "#;
    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn ron_roundtrip_list() -> Result {
    let code = r#"
        [1 2 3 4]
        | to ron
        | from ron
        | $in == [1 2 3 4]
    "#;
    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn ron_roundtrip_empty() -> Result {
    let code = r#"
        []
        | to ron
        | from ron
        | $in == []
    "#;
    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn ron_roundtrip_nested() -> Result {
    let code = r#"
        {a: 1 b: [2 3] c: {d: 4 e: 5}}
        | to ron
        | from ron
        | $in == {a: 1 b: [2 3] c: {d: 4 e: 5}}
    "#;
    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn ron_roundtrip_complex() -> Result {
    let code = r#"
        {a: 1 b: [2 3] c: {d: 4 e: 5} f: "hello" g: true h: null}
        | to ron
        | from ron
        | $in == {a: 1 b: [2 3] c: {d: 4 e: 5} f: "hello" g: true h: null}
    "#;
    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn ron_explicit_map_to_nushell_record() -> Result {
    let code = r#"
        '{"a": 1, "b": [2, 3]}'
        | from ron
        | $in == {a: 1 b: [2 3]}
    "#;
    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn ron_option_some() -> Result {
    let code = r#"
        'Some(42)'
        | from ron
        | $in == 42
    "#;
    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn ron_option_none() -> Result {
    let code = r#"
        'None'
        | from ron
        | $in == null
    "#;
    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn ron_pipeline_roundtrip() -> Result {
    let code = r#"
        [1 2 3]
        | each { |value| $value * 2 }
        | to ron
        | from ron
        | math sum
        | $in == 12
    "#;

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn ron_integration_record_pipeline() -> Result {
    let code = r#"
        '{"users": [{"name": "Ada", "active": true}, {"name": "Grace", "active": false}]}'
        | from ron
        | get users
        | where active
        | get name
        | to ron
        | from ron
        | $in == ['Ada']
    "#;

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn ron_file_open_and_save() -> Result {
    Playground::setup("ron_file_open_and_save", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "input.ron",
            r#"
                {"name": "Ada", "languages": ["Rust", "Nushell"]}
            "#,
        )]);

        let code = r#"
            open input.ron
            | save output.ron
            open output.ron
            | $in == {name: "Ada" languages: ["Rust" "Nushell"]}
        "#;

        test().cwd(dirs.test()).run(code).expect_value_eq(true)
    })
}

#[test]
fn ron_rejects_malformed_syntax() -> Result {
    let code = r#"'[1, 2' | from ron"#;

    assert!(test().run::<Value>(code).is_err());
    Ok(())
}

#[test]
fn ron_rejects_unsupported_type() -> Result {
    let code = r#"{|value| $value} | to ron"#;

    assert!(test().run::<Value>(code).is_err());
    Ok(())
}
