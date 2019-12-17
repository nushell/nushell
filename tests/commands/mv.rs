use nu_test_support::fs::{files_exist_at, Stub::EmptyFile};
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn moves_a_file() {
    Playground::setup("mv_test_1", |dirs, sandbox| {
        sandbox
            .with_files(vec![EmptyFile("andres.txt")])
            .mkdir("expected");

        let original = dirs.test().join("andres.txt");
        let expected = dirs.test().join("expected/yehuda.txt");

        nu!(
            cwd: dirs.test(),
            "mv andres.txt expected/yehuda.txt"
        );

        assert!(!original.exists());
        assert!(expected.exists());
    })
}

#[test]
fn overwrites_if_moving_to_existing_file() {
    Playground::setup("mv_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("andres.txt"), EmptyFile("jonathan.txt")]);

        let original = dirs.test().join("andres.txt");
        let expected = dirs.test().join("jonathan.txt");

        nu!(
            cwd: dirs.test(),
            "mv andres.txt jonathan.txt"
        );

        assert!(!original.exists());
        assert!(expected.exists());
    })
}

#[test]
fn moves_a_directory() {
    Playground::setup("mv_test_3", |dirs, sandbox| {
        sandbox.mkdir("empty_dir");

        let original_dir = dirs.test().join("empty_dir");
        let expected = dirs.test().join("renamed_dir");

        nu!(
            cwd: dirs.test(),
            "mv empty_dir renamed_dir"
        );

        assert!(!original_dir.exists());
        assert!(expected.exists());
    })
}

#[test]
fn moves_the_file_inside_directory_if_path_to_move_is_existing_directory() {
    Playground::setup("mv_test_4", |dirs, sandbox| {
        sandbox
            .with_files(vec![EmptyFile("jonathan.txt")])
            .mkdir("expected");

        let original_dir = dirs.test().join("jonathan.txt");
        let expected = dirs.test().join("expected/jonathan.txt");

        nu!(
            cwd: dirs.test(),
            "mv jonathan.txt expected"
        );

        assert!(!original_dir.exists());
        assert!(expected.exists());
    })
}

#[test]
fn moves_the_directory_inside_directory_if_path_to_move_is_existing_directory() {
    Playground::setup("mv_test_5", |dirs, sandbox| {
        sandbox
            .within("contributors")
            .with_files(vec![EmptyFile("jonathan.txt")])
            .mkdir("expected");

        let original_dir = dirs.test().join("contributors");
        let expected = dirs.test().join("expected/contributors");

        nu!(
            cwd: dirs.test(),
            "mv contributors expected"
        );

        assert!(!original_dir.exists());
        assert!(expected.exists());
    })
}

#[test]
fn moves_the_directory_inside_directory_if_path_to_move_is_nonexistent_directory() {
    Playground::setup("mv_test_6", |dirs, sandbox| {
        sandbox
            .within("contributors")
            .with_files(vec![EmptyFile("jonathan.txt")])
            .mkdir("expected");

        let original_dir = dirs.test().join("contributors");

        nu!(
            cwd: dirs.test(),
            "mv contributors expected/this_dir_exists_now/los_tres_amigos"
        );

        let expected = dirs
            .test()
            .join("expected/this_dir_exists_now/los_tres_amigos");

        assert!(!original_dir.exists());
        assert!(expected.exists());
    })
}

#[test]
fn moves_using_path_with_wildcard() {
    Playground::setup("mv_test_7", |dirs, sandbox| {
        sandbox
            .within("originals")
            .with_files(vec![
                EmptyFile("andres.ini"),
                EmptyFile("caco3_plastics.csv"),
                EmptyFile("cargo_sample.toml"),
                EmptyFile("jonathan.ini"),
                EmptyFile("jonathan.xml"),
                EmptyFile("sgml_description.json"),
                EmptyFile("sample.ini"),
                EmptyFile("utf16.ini"),
                EmptyFile("yehuda.ini"),
            ])
            .mkdir("work_dir")
            .mkdir("expected");

        let work_dir = dirs.test().join("work_dir");
        let expected = dirs.test().join("expected");

        nu!(cwd: work_dir, "mv ../originals/*.ini ../expected");

        assert!(files_exist_at(
            vec!["yehuda.ini", "jonathan.ini", "sample.ini", "andres.ini",],
            expected
        ));
    })
}

#[test]
fn moves_using_a_glob() {
    Playground::setup("mv_test_8", |dirs, sandbox| {
        sandbox
            .within("meals")
            .with_files(vec![
                EmptyFile("arepa.txt"),
                EmptyFile("empanada.txt"),
                EmptyFile("taquiza.txt"),
            ])
            .mkdir("work_dir")
            .mkdir("expected");

        let meal_dir = dirs.test().join("meals");
        let work_dir = dirs.test().join("work_dir");
        let expected = dirs.test().join("expected");

        nu!(cwd: work_dir, "mv ../meals/* ../expected");

        assert!(meal_dir.exists());
        assert!(files_exist_at(
            vec!["arepa.txt", "empanada.txt", "taquiza.txt",],
            expected
        ));
    })
}
