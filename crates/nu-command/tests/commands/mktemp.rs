use nu_test_support::nu;
use nu_test_support::playground::Playground;
use std::path::PathBuf;

#[test]
fn creates_temp_file() {
    Playground::setup("mktemp_test_1", |dirs, _| {
        let output = nu!(
            cwd: dirs.test(),
            "mktemp"
        );
        let loc = PathBuf::from(output.out.clone());
        println!("{:?}", loc);
        assert!(loc.exists());
    })
}

#[test]
fn creates_temp_file_with_suffix() {
    Playground::setup("mktemp_test_2", |dirs, _| {
        let output = nu!(
            cwd: dirs.test(),
            "mktemp --suffix .txt tempfileXXX"
        );
        let loc = PathBuf::from(output.out.clone());
        assert!(loc.exists());
        assert!(loc.is_file());
        assert!(output.out.ends_with(".txt"));
        assert!(output.out.starts_with(dirs.test().to_str().unwrap()));
    })
}

#[test]
fn creates_temp_directory() {
    Playground::setup("mktemp_test_3", |dirs, _| {
        let output = nu!(
            cwd: dirs.test(),
            "mktemp -d"
        );

        let loc = PathBuf::from(output.out);
        assert!(loc.exists());
        assert!(loc.is_dir());
    })
}
