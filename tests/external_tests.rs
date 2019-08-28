mod helpers;

use helpers::in_directory as cwd;

#[test]
fn external_command() {
    let output = nu!(cwd("tests/fixtures"), "echo 1");

    assert!(output.contains("1"));
}
