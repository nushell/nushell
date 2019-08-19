mod helpers;

use h::{in_directory as cwd, Playground, Stub::*};
use helpers as h;
use std::path::{Path, PathBuf};

#[test]
fn knows_the_filesystems_entered() {
    let sandbox = Playground::setup_for("enter_filesystem_sessions_test")
        .within("red_pill")
        .with_files(vec![
            EmptyFile("andres.nu"),
            EmptyFile("jonathan.nu"),
            EmptyFile("yehuda.nu"),
        ])
        .within("blue_pill")
        .with_files(vec![
            EmptyFile("bash.nxt"),
            EmptyFile("korn.nxt"),
            EmptyFile("powedsh.nxt"),
        ])
        .mkdir("expected")
        .test_dir_name();

    let full_path = format!("{}/{}", Playground::root(), sandbox);

    let red_pill_dir = format!("{}/{}", full_path, "red_pill");
    let blue_pill_dir = format!("{}/{}", full_path, "blue_pill");
    let expected = format!("{}/{}", full_path, "expected");
    let expected_recycled = format!("{}/{}", expected, "recycled");

    nu!(
        _output,
        cwd(&full_path),
        r#"
            enter expected
            mkdir recycled
            enter ../red_pill
            mv jonathan.nu ../expected
            enter ../blue_pill
            cp *.nxt ../expected/recycled
            p
            p
            mv ../red_pill/yehuda.nu .
            n
            mv andres.nu ../expected/andres.nu
            exit
            n
            cd ..
            rm red_pill --recursive
            exit
            n
            rm blue_pill --recursive
            exit
        "#
    );

    assert!(!h::dir_exists_at(PathBuf::from(red_pill_dir)));
    assert!(h::files_exist_at(
        vec![
            Path::new("andres.nu"),
            Path::new("jonathan.nu"),
            Path::new("yehuda.nu"),
        ],
        PathBuf::from(&expected)
    ));

    assert!(!h::dir_exists_at(PathBuf::from(blue_pill_dir)));
    assert!(h::files_exist_at(
        vec![
            Path::new("bash.nxt"),
            Path::new("korn.nxt"),
            Path::new("powedsh.nxt"),
        ],
        PathBuf::from(&expected_recycled)
    ));
}
