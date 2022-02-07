use nu_test_support::fs::{files_exist_at, Stub::EmptyFile};
use nu_test_support::nu;
use nu_test_support::playground::Playground;
use std::path::Path;

#[test]
fn knows_the_filesystems_entered() {
    Playground::setup("enter_test_1", |dirs, sandbox| {
        sandbox
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
            .mkdir("expected");

        let red_pill_dir = dirs.test().join("red_pill");
        let blue_pill_dir = dirs.test().join("blue_pill");
        let expected = dirs.test().join("expected");
        let expected_recycled = expected.join("recycled");

        nu!(
            cwd: dirs.test(),
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

        assert!(!red_pill_dir.exists());
        assert!(files_exist_at(
            vec![
                Path::new("andres.nu"),
                Path::new("jonathan.nu"),
                Path::new("yehuda.nu"),
            ],
            expected
        ));

        assert!(!blue_pill_dir.exists());
        assert!(files_exist_at(
            vec![
                Path::new("bash.nxt"),
                Path::new("korn.nxt"),
                Path::new("powedsh.nxt"),
            ],
            expected_recycled
        ));
    })
}
