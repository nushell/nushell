mod helpers;

use h::{in_directory as cwd, Playground, Stub::*};
use helpers as h;

#[test]
fn ls_lists_regular_files() {
    let sandbox = Playground::setup_for("ls_lists_files_test")
        .with_files(vec![
            EmptyFile("yehuda.10.txt"),
            EmptyFile("jonathan.10.txt"),
            EmptyFile("andres.10.txt"),
        ])
        .test_dir_name();

    let full_path = format!("{}/{}", Playground::root(), sandbox);

    nu!(
        output,
        cwd(&full_path),
        r#"ls | get name | lines | split-column "." | get Column2 | str Column2 --to-int | sum | echo $it"#
    );

    assert_eq!(output, "30");
}

#[test]
fn ls_lists_regular_files_using_asterisk_wildcard() {
    let sandbox = Playground::setup_for("ls_asterisk_wildcard_test")
        .with_files(vec![
            EmptyFile("los.1.txt"),
            EmptyFile("tres.1.txt"),
            EmptyFile("amigos.1.txt"),
            EmptyFile("arepas.1.clu"),
        ])
        .test_dir_name();

    let full_path = format!("{}/{}", Playground::root(), sandbox);

    nu!(
        output,
        cwd(&full_path),
        "ls *.txt | get name | lines| split-column \".\" | get Column2 | str Column2 --to-int | sum | echo $it"
    );

    assert_eq!(output, "3");
}

#[test]
fn ls_lists_regular_files_using_question_mark_wildcard() {
    let sandbox = Playground::setup_for("ls_question_mark_wildcard_test")
        .with_files(vec![
            EmptyFile("yehuda.10.txt"),
            EmptyFile("jonathan.10.txt"),
            EmptyFile("andres.10.txt"),
            EmptyFile("chicken_not_to_be_picked_up.100.txt"),
        ])
        .test_dir_name();

    let full_path = format!("{}/{}", Playground::root(), sandbox);

    nu!(
        output,
        cwd(&full_path),
        "ls *.??.txt | get name | lines| split-column \".\" | get Column2 | str Column2 --to-int | sum | echo $it"
    );

    assert_eq!(output, "30");
}
