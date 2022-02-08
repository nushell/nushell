use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn find_with_list_search_with_string() {
    let actual = nu!(
    cwd: ".", pipeline(
    r#"
        [moe larry curly] | find moe | get 0
    "#
    ));

    assert_eq!(actual.out, "moe");
}

#[test]
fn find_with_list_search_with_char() {
    let actual = nu!(
    cwd: ".", pipeline(
    r#"
        [moe larry curly] | find l | to json -r
    "#
    ));

    assert_eq!(actual.out, r#"["larry","curly"]"#);
}

#[test]
fn find_with_list_search_with_number() {
    let actual = nu!(
    cwd: ".", pipeline(
    r#"
        [1 2 3 4 5] | find 3 | get 0
    "#
    ));

    assert_eq!(actual.out, "3");
}

#[test]
fn find_with_string_search_with_string() {
    let actual = nu!(
    cwd: ".", pipeline(
    r#"
        echo Cargo.toml | find toml
    "#
    ));

    assert_eq!(actual.out, "Cargo.toml");
}

#[test]
fn find_with_string_search_with_string_not_found() {
    let actual = nu!(
    cwd: ".", pipeline(
    r#"
        [moe larry curly] | find shemp
    "#
    ));

    assert_eq!(actual.out, "");
}

#[test]
fn find_with_filepath_search_with_string() {
    Playground::setup("filepath_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | get name
                | find arep
                | to json -r
            "#
        ));

        assert_eq!(actual.out, r#"["arepas.clu"]"#);
    })
}

#[test]
fn find_with_filepath_search_with_multiple_patterns() {
    Playground::setup("filepath_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | get name
                | find arep ami
                | to json -r
            "#
        ));

        assert_eq!(actual.out, r#"["amigos.txt","arepas.clu"]"#);
    })
}
