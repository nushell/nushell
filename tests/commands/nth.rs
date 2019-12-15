#[test]
fn selects_a_row() {
    Playground::setup("nth_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("notes.txt"), EmptyFile("arepas.txt")]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | sort-by name
                | nth 0
                | get name
                | echo $it
            "#
        ));

        assert_eq!(actual, "arepas.txt");
    });
}

#[test]
fn selects_many_rows() {
    Playground::setup("nth_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("notes.txt"), EmptyFile("arepas.txt")]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | get name
                | nth 1 0
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "2");
    });
}