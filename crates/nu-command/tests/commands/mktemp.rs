use nu_path::AbsolutePath;
use nu_test_support::playground::Playground;
use nu_test_support::prelude::*;

#[test]
fn creates_temp_file(playground: Playground) -> Result {
    let output: String = test().cwd(playground.path()).run("mktemp")?;
    let loc = AbsolutePath::try_new(&output).unwrap();
    assert!(loc.exists());
    Ok(())
}

#[test]
fn creates_temp_file_with_suffix(playground: Playground) -> Result {
    let output: String = test()
        .cwd(playground.path())
        .run("mktemp --suffix .txt tempfileXXX")?;
    let loc = AbsolutePath::try_new(&output).unwrap();
    assert!(loc.exists());
    assert!(loc.is_file());
    assert!(output.ends_with(".txt"));
    assert!(output.starts_with(playground.path().to_str().unwrap()));
    Ok(())
}

#[test]
fn creates_temp_directory(playground: Playground) -> Result {
    let output: String = test().cwd(playground.path()).run("mktemp -d")?;
    let loc = AbsolutePath::try_new(&output).unwrap();
    assert!(loc.exists());
    assert!(loc.is_dir());
    Ok(())
}

#[test]
fn doesnt_create_temp_file(playground: Playground) -> Result {
    let output: String = test().cwd(playground.path()).run("mktemp --dry")?;
    let loc = AbsolutePath::try_new(&output).unwrap();
    assert!(!loc.exists());
    Ok(())
}
