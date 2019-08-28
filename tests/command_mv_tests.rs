mod helpers;

use h::{in_directory as cwd, Playground, Stub::*};
use helpers as h;

use std::path::{Path, PathBuf};

#[test]
fn moves_a_file() {
    Playground::setup("mv_test_1", |dirs, playground| {
        playground
            .with_files(vec![EmptyFile("andres.txt")])
            .mkdir("expected")
            .test_dir_name();

        let original = dirs.test().join("andres.txt");
        let expected = dirs.test().join("expected/yehuda.txt");

        nu!(dirs.test(), "mv andres.txt expected/yehuda.txt");

        assert!(!h::file_exists_at(original));
        assert!(h::file_exists_at(expected));
    })
}

#[test]
fn overwrites_if_moving_to_existing_file() {
    Playground::setup("mv_test_2", |dirs, playground| {
        playground
            .with_files(vec![EmptyFile("andres.txt"), EmptyFile("jonathan.txt")])
            .test_dir_name();

        let original = dirs.test().join("andres.txt");
        let expected = dirs.test().join("jonathan.txt");

        nu!(dirs.test(), "mv andres.txt jonathan.txt");

        assert!(!h::file_exists_at(original));
        assert!(h::file_exists_at(expected));
    })
}

#[test]
fn moves_a_directory() {
    Playground::setup("mv_test_3", |dirs, playground| {
        playground.mkdir("empty_dir");

        let original_dir = dirs.test().join("empty_dir");
        let expected = dirs.test().join("renamed_dir");

        nu!(dirs.test(), "mv empty_dir renamed_dir");

        assert!(!h::dir_exists_at(original_dir));
        assert!(h::dir_exists_at(expected));
    })
}

#[test]
fn moves_the_file_inside_directory_if_path_to_move_is_existing_directory() {
    Playground::setup("mv_test_4", |dirs, playground| {
        playground
            .with_files(vec![EmptyFile("jonathan.txt")])
            .mkdir("expected")
            .test_dir_name();

        let original_dir = dirs.test().join("jonathan.txt");
        let expected = dirs.test().join("expected/jonathan.txt");

        nu!(dirs.test(), "mv jonathan.txt expected");

        assert!(!h::file_exists_at(original_dir));
        assert!(h::file_exists_at(expected));
    })
}

#[test]
fn moves_the_directory_inside_directory_if_path_to_move_is_existing_directory() {
    Playground::setup("mv_test_5", |dirs, playground| {
        playground
            .within("contributors")
            .with_files(vec![EmptyFile("jonathan.txt")])
            .mkdir("expected")
            .test_dir_name();

        let original_dir = dirs.test().join("contributors");
        let expected = dirs.test().join("expected/contributors");

        nu!(dirs.test(), "mv contributors expected");

        assert!(!h::dir_exists_at(original_dir));
        assert!(h::file_exists_at(expected));
    })
}

#[test]
fn moves_the_directory_inside_directory_if_path_to_move_is_nonexistent_directory() {
    Playground::setup("mv_test_6", |dirs, playground| {
        playground
            .within("contributors")
            .with_files(vec![EmptyFile("jonathan.txt")])
            .mkdir("expected")
            .test_dir_name();

        let original_dir = dirs.test().join("contributors");

        nu!(
            dirs.test(),
            "mv contributors expected/this_dir_exists_now/los_tres_amigos"
        );

        let expected = dirs
            .test()
            .join("expected/this_dir_exists_now/los_tres_amigos");

        assert!(!h::dir_exists_at(original_dir));
        assert!(h::file_exists_at(expected));
    })
}

#[test]
fn moves_using_path_with_wildcard() {
    Playground::setup("mv_test_7", |dirs, playground| {
        playground
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
            .mkdir("expected")
            .test_dir_name();

        let work_dir = dirs.test().join("work_dir");
        let expected = dirs.test().join("expected");

        nu!(work_dir, "mv ../originals/*.ini ../expected");

        assert!(h::files_exist_at(
            vec!["yehuda.ini", "jonathan.ini", "sample.ini", "andres.ini",],
            expected
        ));
    })
}

#[test]
fn moves_using_a_glob() {
    Playground::setup("mv_test_8", |dirs, playground| {
        playground
            .within("meals")
            .with_files(vec![
                EmptyFile("arepa.txt"),
                EmptyFile("empanada.txt"),
                EmptyFile("taquiza.txt"),
            ])
            .mkdir("work_dir")
            .mkdir("expected")
            .test_dir_name();

        let meal_dir = dirs.test().join("meals");
        let work_dir = dirs.test().join("work_dir");
        let expected = dirs.test().join("expected");

        nu!(work_dir, "mv ../meals/* ../expected");

        assert!(h::dir_exists_at(meal_dir));
        assert!(h::files_exist_at(
            vec!["arepa.txt", "empanada.txt", "taquiza.txt",],
            expected
        ));
    })
}
