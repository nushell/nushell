mod helpers;

use helpers::in_directory as cwd;

#[test]
fn external_command() {
    // Echo should exist on all currently supported platforms. A better approach might
    // be to generate a dummy executable as part of the tests with known semantics.
    nu!(output, cwd("tests/fixtures"), "echo 1");

    assert!(output.contains("1"));
}
