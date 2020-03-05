use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn creates_a_file_when_it_doesnt_exist() {
    Playground::setup("create_test_1", |dirs, _sandbox| {
        nu!(
            cwd: dirs.test(),
            "touch i_will_be_created.txt"
        );

        let path = dirs.test().join("i_will_be_created.txt");

        assert!(path.exists());
    })
}
