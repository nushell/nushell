use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[cfg(windows)]
#[test]
fn create() {
    Playground::setup("symbol_create", |dirs, _| {
        let _ = nu!(cwd: dirs.test(), "touch test_file");
        #[cfg(windows)]
        let _ = nu!(cwd: dirs.test(), "makelink test_file test_symbol");
        #[cfg(not(windows))]
        let _ = nu!(cwd: dirs.test(), "ln -s test_file test_symbol");
        let file_path = dirs.test().join("test_file");
        let symbol_path = dirs.test().join("test_symbol");
        assert!(symbol_path
            .metadata()
            .map(|x| x.is_symbol())
            .unwrap_or_default());
        assert!(file_path
            .metadata()
            .map(|x| x.is_file())
            .unwrap_or_default());
    })
}

#[test]
fn remove() {
    Playground::setup("symbol_remove", |dirs, sandbox| {
        // easiest way to make a symlink on windows is to just do it via nu
        let _ = nu!(cwd: dirs.test(), "touch test_file");
        #[cfg(windows)]
        let _ = nu!(cwd: dirs.test(), "makelink test_file test_symbol");
        #[cfg(not(windows))]
        let _ = nu!(cwd: dirs.test(), "ln -s test_file test_symbol");
        let file_path = dirs.test().join("test_file");
        let symbol_path = dirs.test().join("test_symbol");
        let _ = nu!(cwd: dirs.test(), "rm test_symbol");
        assert!(symbol_path.exists());
        assert!(file_path
            .metadata()
            .map(|x| x.is_file())
            .unwrap_or_default());
    })
}
