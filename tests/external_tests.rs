mod helpers;

use helpers::in_directory as cwd;

#[test]
fn external_command() {
    nu!(output, cwd("tests/fixtures"), "echo 1");

    assert!(output.contains("1"));
}
