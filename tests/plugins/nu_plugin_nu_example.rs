use assert_cmd::Command;
use nu_parser::escape_quote_string;

#[test]
fn call() {
    let path = nu_path::canonicalize_with(
        "crates/nu_plugin_nu_example/nu_plugin_nu_example.nu",
        nu_test_support::fs::root(),
    )
    .expect("failed to find nu_plugin_nu_example.nu");

    let assert = Command::new(nu_test_support::fs::executable_path())
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--plugins",
            &format!("[{}]", escape_quote_string(&path.to_string_lossy())),
            "--commands",
            "nu_plugin_nu_example 4242 teststring",
        ])
        .assert()
        .success();

    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("one"));
    assert!(stdout.contains("two"));
    assert!(stdout.contains("three"));
    assert!(stderr.contains("name: nu_plugin_nu_example"));
    assert!(stderr.contains("4242"));
    assert!(stderr.contains("teststring"));
}
