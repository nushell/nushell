use nu_test_support::{nu, pipeline};

#[test]
fn config_add_unsupported_key() {
    let actual = nu!(cwd: "crates/nu-utils/src/sample_config", pipeline(r#" source default_config.nu; $env.config.foo = 2 "#));

    assert!(actual
        .err
        .contains("$env.config.foo is an unknown config setting"));
}

#[test]
fn config_add_unsupported_type() {
    let actual = nu!(cwd: "crates/nu-utils/src/sample_config", pipeline(r#" source default_config.nu; $env.config.ls = '' "#));

    assert!(actual.err.contains("should be a record"));
}

#[test]
fn config_add_unsupported_value() {
    let actual = nu!(cwd: "crates/nu-utils/src/sample_config", pipeline(r#" source default_config.nu; $env.config.history.file_format = ''; "#));

    println!("{:?}", actual.out);
    assert!(actual.err.contains(
        "unrecognized $env.config.history.file_format ''; expected either 'sqlite' or 'plaintext'"
    ));
}

#[test]
fn config_unsupported_key_reverted() {
    let actual = nu!(cwd: "crates/nu-utils/src/sample_config", pipeline(r#" source default_config.nu; do -i { $env.config.foo = 1 | print ('foo' in $env.config) }"#));

    assert_eq!(actual.out, "false");
}

#[test]
fn config_unsupported_type_reverted() {
    let actual = nu!(cwd: "crates/nu-utils/src/sample_config", pipeline(r#" source default_config.nu; do -i { $env.config.ls = '' | print ($env.config.ls | describe) }"#));

    assert!(actual.out.starts_with("record"));
}

#[test]
fn config_unsupported_value_reverted() {
    let actual = nu!(cwd: "crates/nu-utils/src/sample_config", pipeline(r#" source default_config.nu; $env.config.history.file_format = 'plaintext'; do -i { $env.config.history.file_format = ''; } $env.config.history.file_format | to json"#));

    assert_eq!(actual.out, "\"plaintext\"");
}
