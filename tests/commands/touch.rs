use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn adds_a_file() {
    Playground::setup("add_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("i_will_be_created.txt")]);

        nu!(
            cwd: dirs.root(),
            "touch touch_test/i_will_be_created.txt"
        );

        let path = dirs.test().join("i_will_be_created.txt");

        assert!(path.exists());
    })
}
