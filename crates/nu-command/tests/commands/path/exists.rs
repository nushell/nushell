use nu_test_support::{fs::Stub::EmptyFile, prelude::*};

#[test]
fn checks_if_existing_file_exists(playground: Playground) -> Result {
    playground.empty_file("spam.txt")?;

    let outcome: bool = test()
        .cwd(playground.path())
        .run("echo spam.txt | path exists")?;
    assert!(outcome);
    Ok(())
}

#[test]
fn checks_if_missing_file_exists(playground: Playground) -> Result {
    let outcome: bool = test()
        .cwd(playground.path())
        .run("echo spam.txt | path exists")?;
    assert!(!outcome);
    Ok(())
}

#[test]
fn checks_if_dot_exists(playground: Playground) -> Result {
    let outcome: bool = test()
        .cwd(playground.path())
        .run("echo '.' | path exists")?;
    assert!(outcome);
    Ok(())
}

#[test]
fn checks_if_double_dot_exists(playground: Playground) -> Result {
    let outcome: bool = test()
        .cwd(playground.path())
        .run("echo '..' | path exists")?;
    assert!(outcome);
    Ok(())
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
fn path_exists_under_a_non_directory(playground: Playground) -> Result {
    let outcome: bool = test()
        .cwd(playground.path())
        .run("touch test_file; 'test_file/aaa' | path exists")?;
    assert!(!outcome);
    Ok(())
}

#[test]
fn test_check_symlink_exists(playground: Playground) -> Result {
    let symlink_target = "symlink_target";
    let symlink = "symlink";
    #[cfg(not(windows))]
    std::os::unix::fs::symlink(
        playground.path().join(symlink_target),
        playground.path().join(symlink),
    )
    .unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(
        playground.path().join(symlink_target),
        playground.path().join(symlink),
    )
    .unwrap();

    let outcome: bool = test()
        .cwd(playground.path())
        .run("'symlink_target' | path exists")?;
    assert!(!outcome);
    let outcome: bool = test()
        .cwd(playground.path())
        .run("'symlink' | path exists")?;
    assert!(!outcome);
    let outcome: bool = test()
        .cwd(playground.path())
        .run("'symlink' | path exists -n")?;
    assert!(outcome);
    Ok(())
}
