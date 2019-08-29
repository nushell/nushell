mod helpers;

use helpers as h;
use helpers::{Playground, Stub::*};

#[test]
fn ls_lists_regular_files() {
    Playground::setup("ls_test_1", |dirs, sandbox| {
        sandbox
            .with_files(vec![
                EmptyFile("yehuda.10.txt"),
                EmptyFile("jonathan.10.txt"),
                EmptyFile("andres.10.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                ls 
                | get name 
                | lines 
                | split-column "." 
                | get Column2 
                | str --to-int 
                | sum 
                | echo $it
            "#
        ));

        assert_eq!(actual, "30");
    })
}

#[test]
fn ls_lists_regular_files_using_asterisk_wildcard() {
    Playground::setup("ls_test_2", |dirs, sandbox| {
        sandbox
            .with_files(vec![
                EmptyFile("los.1.txt"),
                EmptyFile("tres.1.txt"),
                EmptyFile("amigos.1.txt"),
                EmptyFile("arepas.1.clu"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                ls *.txt 
                | get name 
                | lines 
                | split-column "." 
                | get Column2 
                | str --to-int 
                | sum 
                | echo $it
            "#
        ));

        assert_eq!(actual, "3");
    })
}

#[test]
fn ls_lists_regular_files_using_question_mark_wildcard() {
    Playground::setup("ls_test_3", |dirs, sandbox| {
        sandbox
            .with_files(vec![
                EmptyFile("yehuda.10.txt"),
                EmptyFile("jonathan.10.txt"),
                EmptyFile("andres.10.txt"),
                EmptyFile("chicken_not_to_be_picked_up.100.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                ls *.??.txt 
                | get name 
                | lines 
                | split-column "." 
                | get Column2 
                | str --to-int 
                | sum 
                | echo $it
            "#
        ));

        assert_eq!(actual, "30");
    })
}
