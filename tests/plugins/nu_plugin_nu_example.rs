use std::process::Command;

use nu_test_support::prelude::*;

#[test]
#[cfg_attr(all(windows, ci), ignore = "likes to fail on Windows in CI")]
#[deps(NU)]
fn call() -> Result {
    let output = Command::new(NU.path())
        .env(nu_utils::consts::NATIVE_PATH_ENV_VAR, NU.bin_dir())
        .current_dir(WORKSPACE_ROOT.as_path())
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--plugins",
            "crates/nu_plugin_nu_example/nu_plugin_nu_example.nu",
            "--commands",
            "nu_plugin_nu_example 4242 teststring",
        ])
        .output()?;

    let stdout = str::from_utf8(&output.stdout).expect("stdout is utf8");
    let stderr = str::from_utf8(&output.stderr).expect("stderr is utf8");
    assert!(output.status.success(), "{stderr}");
    assert_contains("one", stdout);
    assert_contains("two", stdout);
    assert_contains("three", stdout);
    assert_contains("name: nu_plugin_nu_example", stderr);
    assert_contains("4242", stderr);
    assert_contains("teststring", stderr);
    Ok(())
}
