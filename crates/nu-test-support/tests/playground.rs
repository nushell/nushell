use nu_test_support::prelude::*;

#[test]
fn names_temp_dir_with_test_slug() -> Result {
    let playground = Playground::new(
        "crate::tests::playground::case",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_CRATE_NAME"),
    )?;
    let name = playground.path().file_name().unwrap().to_string_lossy();

    assert!(name.starts_with("tests.playground.case-"));

    playground.close()?;
    Ok(())
}

#[test]
fn names_package_dir_with_pkg_and_crate_name() -> Result {
    let playground = Playground::new("crate::tests::playground::case", "pkg-name", "crate_name")?;

    let package_dir = playground.path().parent().unwrap().file_name().unwrap();
    assert_eq!(package_dir, "pkg-name.crate_name");

    playground.close()?;
    Ok(())
}

#[track_caller]
fn assert_invalid_path<T>(result: Result<T, PlaygroundError>, expected: &str) {
    match result {
        Ok(_) => panic!("expected invalid playground path"),
        Err(err) => assert_contains(expected, err.to_string()),
    }
}

#[test]
#[expect(unreachable_code)]
fn rejects_empty_paths() -> Result {
    let playground = Playground::new(
        "crate::tests::playground::rejects_empty_paths",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_CRATE_NAME"),
    )?;

    assert_invalid_path(playground.dir(""), "path is empty");
    assert_invalid_path(playground.empty_file(""), "path is empty");
    assert_invalid_path(playground.file("", "contents"), "path is empty");
    assert_invalid_path(
        playground.at("", |_| {
            panic!("empty at path should not call closure");
            Ok(())
        }),
        "path is empty",
    );

    playground.close()?;
    Ok(())
}

#[test]
#[expect(unreachable_code)]
fn rejects_root_only_paths() -> Result {
    let playground = Playground::new(
        "crate::tests::playground::rejects_root_only_paths",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_CRATE_NAME"),
    )?;

    assert_invalid_path(playground.dir("/"), "path is empty");
    assert_invalid_path(playground.empty_file("/"), "path is empty");
    assert_invalid_path(playground.file("/", "contents"), "path is empty");
    assert_invalid_path(
        playground.at("/", |_| {
            panic!("root-only at path should not call closure");
            Ok(())
        }),
        "path is empty",
    );

    playground.close()?;
    Ok(())
}

#[test]
#[expect(unreachable_code)]
fn rejects_parent_dir_paths() -> Result {
    let playground = Playground::new(
        "crate::tests::playground::rejects_parent_dir_paths",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_CRATE_NAME"),
    )?;

    assert_invalid_path(playground.dir(".."), "path includes parent dir");
    assert_invalid_path(playground.empty_file("../file"), "path includes parent dir");
    assert_invalid_path(
        playground.file("nested/../file", "contents"),
        "path includes parent dir",
    );
    assert_invalid_path(
        playground.at("nested/..", |_| {
            panic!("parent dir at path should not call closure");
            Ok(())
        }),
        "path includes parent dir",
    );

    playground.close()?;
    Ok(())
}

#[test]
fn treats_leading_root_as_playground_relative() -> Result {
    let playground = Playground::new(
        "crate::tests::playground::treats_leading_root_as_playground_relative",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_CRATE_NAME"),
    )?;

    let dir = playground.dir("/abc/def")?;
    let empty_file = playground.empty_file("/abc/def/file.empty")?;
    let file = playground.file("/abc/file.txt", "contents")?;
    playground.at("/abc/nested", |at| at.empty_file("child.empty"))?;

    assert!(dir.is_dir());
    assert!(empty_file.is_file());
    assert_eq!(std::fs::read_to_string(file)?, "contents");
    assert!(playground.path().join("abc/nested/child.empty").is_file());

    playground.close()?;
    Ok(())
}

#[test]
fn permits_current_dir_components() -> Result {
    let playground = Playground::new(
        "crate::tests::playground::permits_current_dir_components",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_CRATE_NAME"),
    )?;

    let dir = playground.dir("./abc")?;
    let file = playground.file("./abc/./file.txt", "contents")?;

    assert!(dir.is_dir());
    assert_eq!(std::fs::read_to_string(file)?, "contents");

    playground.close()?;
    Ok(())
}
