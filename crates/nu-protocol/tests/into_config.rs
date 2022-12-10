use nu_test_support::{nu, nu_repl_code};

#[test]
fn config_is_mutable() {
    let actual = nu!(cwd: ".", nu_repl_code(&[r"let-env config = { ls: { clickable_links: true } }",
        "$env.config.ls.clickable_links = false;",
        "$env.config.ls.clickable_links"]));

    assert_eq!(actual.out, "false");
}

#[test]
fn config_preserved_after_do() {
    let actual = nu!(cwd: ".", nu_repl_code(&[r"let-env config = { ls: { clickable_links: true } }",
        "do -i { $env.config.ls.clickable_links = false }",
        "$env.config.ls.clickable_links"]));

    assert_eq!(actual.out, "true");
}

#[test]
fn config_affected_when_mutated() {
    let actual = nu!(cwd: ".", nu_repl_code(&[r#"let-env config = { filesize: { metric: false, format:"auto" } }"#,
        r#"$env.config = { filesize: { metric: true, format:"auto" } }"#,
        "20mib | into string"]));

    assert_eq!(actual.out, "21.0 MB");
}

#[test]
fn config_affected_when_deep_mutated() {
    let actual = nu!(cwd: "crates/nu-utils/src/sample_config", nu_repl_code(&[
        r#"source default_config.nu"#,
        r#"$env.config.filesize.metric = true"#,
        r#"20mib | into string"#]));

    assert_eq!(actual.out, "21.0 MB");
}

#[test]
fn config_add_unsupported_key() {
    let actual = nu!(cwd: "crates/nu-utils/src/sample_config", nu_repl_code(&[
        r#"source default_config.nu"#,
        r#"$env.config.foo = 2"#,
        r#";"#]));

    assert!(actual
        .err
        .contains("$env.config.foo is an unknown config setting"));
}

#[test]
fn config_add_unsupported_type() {
    let actual = nu!(cwd: "crates/nu-utils/src/sample_config", nu_repl_code(&[r#"source default_config.nu"#,
        r#"$env.config.ls = '' "#,
        r#";"#]));

    assert!(actual.err.contains("should be a record"));
}

#[test]
fn config_add_unsupported_value() {
    let actual = nu!(cwd: "crates/nu-utils/src/sample_config", nu_repl_code(&[r#"source default_config.nu"#,
        r#"$env.config.history.file_format = ''"#,
        r#";"#]));

    assert!(actual.err.contains(
        "unrecognized $env.config.history.file_format ''; expected either 'sqlite' or 'plaintext'"
    ));
}

#[test]
#[ignore = "Figure out how to make test_bins::nu_repl() continue execution after shell errors"]
fn config_unsupported_key_reverted() {
    let actual = nu!(cwd: "crates/nu-utils/src/sample_config", nu_repl_code(&[r#"source default_config.nu"#,
        r#"$env.config.foo = 1"#,
        r#"'foo' in $env.config"#]));

    assert_eq!(actual.out, "false");
}

#[test]
#[ignore = "Figure out how to make test_bins::nu_repl() continue execution after shell errors"]
fn config_unsupported_type_reverted() {
    let actual = nu!(cwd: "crates/nu-utils/src/sample_config", nu_repl_code(&[r#" source default_config.nu"#,
        r#"$env.config.ls = ''"#,
        r#"$env.config.ls | describe"#]));

    assert_eq!(actual.out, "record");
}

#[test]
#[ignore = "Figure out how to make test_bins::nu_repl() continue execution after errors"]
fn config_unsupported_value_reverted() {
    let actual = nu!(cwd: "crates/nu-utils/src/sample_config", nu_repl_code(&[r#" source default_config.nu"#,
        r#"$env.config.history.file_format = 'plaintext'"#,
        r#"$env.config.history.file_format = ''"#,
        r#"$env.config.history.file_format | to json"#]));

    assert_eq!(actual.out, "\"plaintext\"");
}
