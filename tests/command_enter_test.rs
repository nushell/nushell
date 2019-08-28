mod helpers;

use h::{Playground, Stub::*};
use helpers as h;
use std::path::{Path, PathBuf};

#[test]
fn knows_the_filesystems_entered() {
    Playground::setup("enter_filesystem_sessions_test", |dirs, playground| {
        playground
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

        let red_pill_dir = dirs.test().join("red_pill");
        let blue_pill_dir = dirs.test().join("blue_pill");
        let expected = dirs.test().join("expected");
        let expected_recycled = expected.join("recycled");

        nu!(
            dirs.test(),
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
    })
}
