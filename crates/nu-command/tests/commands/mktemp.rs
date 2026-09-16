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
fn doesnt_create_temp_file() -> Result {
    Playground::setup("mktemp_test_1", |dirs, _| {
        let output: String = test().cwd(dirs.test()).run("mktemp --dry")?;
        let loc = AbsolutePath::try_new(&output).unwrap();
        assert!(!loc.exists());
        Ok(())
    })
}

#[test]
fn error_message_survives_prior_uutils_command() -> Result {
    // uucore initializes its translation bundles once per thread for the
    // first util that asks. Nushell runs many utils per thread, so without
    // per-invocation re-selection a prior `uname` left mktemp error ids
    // untranslated (issue #19004: raw `mktemp-error-too-few-xs` surfaced
    // instead of the message). Both runs share one tester, hence one
    // thread, mirroring a single `nu` session.
    let mut t = test();
    let _: Value = t.run("uname")?;
    let err = t.run("mktemp foo").expect_error()?;
    let msg = err.generic_msg()?;
    assert!(
        msg.contains("too few X's"),
        "expected translated message, got: {msg}"
    );
    assert!(
        !msg.contains("mktemp-error-too-few-xs"),
        "raw message id leaked through: {msg}"
    );
    Ok(())
}
