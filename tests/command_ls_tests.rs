mod helpers;

use h::{in_directory as cwd, Playground, Stub::*};
use helpers as h;

#[test]
fn ls_lists_regular_files() {
    Playground::setup("ls_lists_files_test", |dirs, playground| {
        playground
            .with_files(vec![
                EmptyFile("yehuda.10.txt"),
                EmptyFile("jonathan.10.txt"),
                EmptyFile("andres.10.txt"),
            ])
            .test_dir_name();

        let output = nu!(
            dirs.test(),
            r#"ls | get name | lines | split-column "." | get Column2 | str --to-int | sum | echo $it"#
        );

        assert_eq!(output, "30");
    })
}

#[test]
fn ls_lists_regular_files_using_asterisk_wildcard() {
    Playground::setup("ls_asterisk_wildcard_test", |dirs, playground| {
        playground
            .with_files(vec![
                EmptyFile("los.1.txt"),
                EmptyFile("tres.1.txt"),
                EmptyFile("amigos.1.txt"),
                EmptyFile("arepas.1.clu"),
            ])
            .test_dir_name();

        let output = nu!(
            dirs.test(),
            r#"ls *.txt | get name | lines| split-column "." | get Column2 | str --to-int | sum | echo $it"#
        );

        assert_eq!(output, "3");
    })
}

#[test]
fn ls_lists_regular_files_using_question_mark_wildcard() {
    Playground::setup("ls_question_mark_wildcard_test", |dirs, playground| {
        playground
            .with_files(vec![
                EmptyFile("yehuda.10.txt"),
                EmptyFile("jonathan.10.txt"),
                EmptyFile("andres.10.txt"),
                EmptyFile("chicken_not_to_be_picked_up.100.txt"),
            ])
            .test_dir_name();

        let output = nu!(
        dirs.test(),
        r#"ls *.??.txt | get name | lines| split-column "." | get Column2 | str --to-int | sum | echo $it"#
    );

        assert_eq!(output, "30");
    })
}
