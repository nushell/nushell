mod helpers;

use helpers::in_directory as cwd;
use helpers::Playground;

#[test]
fn cd_directory_not_found() {
    let sandbox = Playground::setup_for("cd_directory_not_found_test").test_dir_name();

    let full_path = format!("{}/{}", Playground::root(), sandbox);

    nu_error!(output, cwd(&full_path), "cd dir_that_does_not_exist");

    assert!(output.contains("dir_that_does_not_exist"));
    assert!(output.contains("directory not found"));
}