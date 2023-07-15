use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn create() {
    Playground::setup("file_create", |dirs, _| {
        let _ = nu!(cwd: dirs.test(), "touch test_file");
        let file_path = dirs.test().join("test_file");
        assert!(file_path
            .metadata()
            .map(|x| x.is_file())
            .unwrap_or_default());
    })
}

#[test]
fn remove() {
    Playground::setup("file_remove", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent("test_file", "stuff")]);
        let _ = nu!(cwd: dirs.test(), "rm test_file");
        let file_path = dirs.test().join("test_file");
        assert!(!file_path.exists());
    })
}
