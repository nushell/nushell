use assert_cmd::Command;

#[test]
fn call() {
    // Add the `nu` binaries to the path env
    let path_env = std::env::join_paths(
        std::iter::once(nu_test_support::fs::binaries().into()).chain(
            std::env::var_os(nu_test_support::NATIVE_PATH_ENV_VAR)
                .as_deref()
                .map(std::env::split_paths)
                .into_iter()
                .flatten(),
        ),
    )
    .expect("failed to make path var");

    let assert = Command::new(nu_test_support::fs::executable_path())
        .env(nu_test_support::NATIVE_PATH_ENV_VAR, path_env)
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--plugins",
            &format!(
                "[crates{0}nu_plugin_nu_example{0}nu_plugin_nu_example.nu]",
                std::path::MAIN_SEPARATOR
            ),
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
