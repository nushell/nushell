use nu_path::AbsolutePath;
use nu_test_support::playground::Playground;
use nu_test_support::prelude::*;

#[test]
fn creates_temp_file() -> Result {
    Playground::setup("mktemp_test_1", |dirs, _| {
        let output: String = test().cwd(dirs.test()).run("mktemp")?;
        let loc = AbsolutePath::try_new(&output).unwrap();
        assert!(loc.exists());
        Ok(())
    })
}

#[test]
fn creates_temp_file_with_suffix() -> Result {
    Playground::setup("mktemp_test_2", |dirs, _| {
        let output: String = test()
            .cwd(dirs.test())
            .run("mktemp --suffix .txt tempfileXXX")?;
        let loc = AbsolutePath::try_new(&output).unwrap();
        assert!(loc.exists());
        assert!(loc.is_file());
        assert!(output.ends_with(".txt"));
        assert!(output.starts_with(dirs.test().to_str().unwrap()));
        Ok(())
    })
}

#[test]
fn creates_temp_directory() -> Result {
    Playground::setup("mktemp_test_3", |dirs, _| {
        let output: String = test().cwd(dirs.test()).run("mktemp -d")?;
        let loc = AbsolutePath::try_new(&output).unwrap();
        assert!(loc.exists());
        assert!(loc.is_dir());
        Ok(())
    })
}

#[test]
fn creates_temp_directory_relative_to_cwd_with_template() -> Result {
    // A template is interpreted relative to the cwd even with
    // `--directory`; only `-t`/`-p`/no-template uses the system temp dir.
    Playground::setup("mktemp_test_4", |dirs, _| {
        let output: String = test()
            .cwd(dirs.test())
            .run("mktemp --directory tmpdir.XXX")?;
        let loc = AbsolutePath::try_new(&output).unwrap();
        assert!(loc.exists());
        assert!(loc.is_dir());
        assert!(output.starts_with(dirs.test().to_str().unwrap()));
        Ok(())
    })
}

#[test]
fn doesnt_create_temp_file() -> Result {
    Playground::setup("mktemp_test_1", |dirs, _| {
        let output: String = test().cwd(dirs.test()).run("mktemp --dry")?;
        let loc = AbsolutePath::try_new(&output).unwrap();
        assert!(!loc.exists());
        Ok(())
    })
}
