use nu_test_support::prelude::*;

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
    let playground = Playground::new(module_path!())?;

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
fn rejects_parent_dir_paths() -> Result {
    let playground = Playground::new(module_path!())?;

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
    let playground = Playground::new(module_path!())?;

    playground.dir("/abc/def")?;
    playground.empty_file("/abc/def/file.empty")?;
    playground.file("/abc/file.txt", "contents")?;
    playground.at("/abc/nested", |at| {
        at.empty_file("child.empty")?;
        Ok(())
    })?;

    assert!(playground.path().join("abc/def").is_dir());
    assert!(playground.path().join("abc/def/file.empty").is_file());
    assert_eq!(
        std::fs::read_to_string(playground.path().join("abc/file.txt"))?,
        "contents"
    );
    assert!(playground.path().join("abc/nested/child.empty").is_file());

    playground.close()?;
    Ok(())
}

#[test]
fn permits_current_dir_components() -> Result {
    let playground = Playground::new(module_path!())?;

    playground.dir("./abc")?;
    playground.file("./abc/./file.txt", "contents")?;

    assert_eq!(
        std::fs::read_to_string(playground.path().join("abc/file.txt"))?,
        "contents"
    );

    playground.close()?;
    Ok(())
}
