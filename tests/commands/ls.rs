use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn lists_regular_files() {
    Playground::setup("ls_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yehuda.txt"),
            EmptyFile("jonathan.txt"),
            EmptyFile("andres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "3");
    })
}

#[test]
fn lists_regular_files_using_asterisk_wildcard() {
    Playground::setup("ls_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls *.txt
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "3");
    })
}

#[test]
fn lists_regular_files_using_question_mark_wildcard() {
    Playground::setup("ls_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yehuda.10.txt"),
            EmptyFile("jonathan.10.txt"),
            EmptyFile("andres.10.txt"),
            EmptyFile("chicken_not_to_be_picked_up.100.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls *.??.txt
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "3");
    })
}

#[test]
fn lists_all_files_in_directories_from_stream() {
    Playground::setup("ls_test_4", |dirs, sandbox| {
        sandbox.mkdir("dir_a").mkdir("dir_b").with_files(vec![
            EmptyFile("root1.txt"),
            EmptyFile("root2.txt"),
            EmptyFile("dir_a/yehuda.10.txt"),
            EmptyFile("dir_a/jonathan.10.txt"),
            EmptyFile("dir_b/andres.10.txt"),
            EmptyFile("dir_b/chicken_not_to_be_picked_up.100.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                echo dir_a dir_b
                | ls $it
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "4");
    })
}
