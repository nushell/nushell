use nu_test_support::{fs::Stub::EmptyFile, prelude::*};

#[test]
fn checks_if_existing_file_exists() -> Result {
    Playground::setup("path_exists_1", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("spam.txt")]);

        let outcome: bool = test().cwd(dirs.test()).run("echo spam.txt | path exists")?;
        assert!(outcome);
        Ok(())
    })
}

#[test]
fn checks_if_missing_file_exists() -> Result {
    Playground::setup("path_exists_2", |dirs, _| {
        let outcome: bool = test().cwd(dirs.test()).run("echo spam.txt | path exists")?;
        assert!(!outcome);
        Ok(())
    })
}

#[test]
fn checks_if_dot_exists() -> Result {
    Playground::setup("path_exists_3", |dirs, _| {
        let outcome: bool = test().cwd(dirs.test()).run("echo '.' | path exists")?;
        assert!(outcome);
        Ok(())
    })
}

#[test]
fn checks_if_double_dot_exists() -> Result {
    Playground::setup("path_exists_4", |dirs, _| {
        let outcome: bool = test().cwd(dirs.test()).run("echo '..' | path exists")?;
        assert!(outcome);
        Ok(())
    })
}

#[test]
fn checks_tilde_relative_path_exists() -> Result {
    let outcome: bool = test().run("'~' | path exists")?;
    assert!(outcome);
    Ok(())
}

#[test]
fn const_path_exists() -> Result {
    let outcome: bool = test().run("const exists = ('~' | path exists); $exists")?;
    assert!(outcome);
    Ok(())
}

#[test]
fn path_exists_under_a_non_directory() -> Result {
    Playground::setup("path_exists_6", |dirs, _| {
        let outcome: bool = test()
            .cwd(dirs.test())
            .run("touch test_file; 'test_file/aaa' | path exists")?;
        assert!(!outcome);
        Ok(())
    })
}

#[test]
fn test_check_symlink_exists() -> Result {
    let symlink_target = "symlink_target";
    let symlink = "symlink";
    Playground::setup("path_exists_5", |dirs, _| {
        #[cfg(not(windows))]
        std::os::unix::fs::symlink(dirs.test().join(symlink_target), dirs.test().join(symlink))
            .unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(
            dirs.test().join(symlink_target),
            dirs.test().join(symlink),
        )
        .unwrap();

        let outcome: bool = test()
            .cwd(dirs.test())
            .run("'symlink_target' | path exists")?;
        assert!(!outcome);
        let outcome: bool = test().cwd(dirs.test()).run("'symlink' | path exists")?;
        assert!(!outcome);
        let outcome: bool = test().cwd(dirs.test()).run("'symlink' | path exists -n")?;
        assert!(outcome);
        Ok(())
    })
}
