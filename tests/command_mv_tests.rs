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

// #[test]
// fn moves_a_directory() {
//     let sandbox = Playground::setup_for("mv_test_3")
//         .mkdir("empty_dir")
//         .test_dir_name();

//     let full_path = format!("{}/{}", Playground::root(), sandbox);
//     let original_dir = format!("{}/{}", full_path, "empty_dir");
//     let expected = format!("{}/{}", full_path, "renamed_dir");

//     nu!(_output, cwd(&full_path), "mv empty_dir renamed_dir");

//     assert!(!h::dir_exists_at(PathBuf::from(original_dir)));
//     assert!(h::dir_exists_at(PathBuf::from(expected)));
// }

// #[test]
// fn moves_the_file_inside_directory_if_path_to_move_is_existing_directory() {
//     let sandbox = Playground::setup_for("mv_test_4")
//         .with_files(vec![EmptyFile("jonathan.txt")])
//         .mkdir("expected")
//         .test_dir_name();

//     let full_path = format!("{}/{}", Playground::root(), sandbox);
//     let original_dir = format!("{}/{}", full_path, "jonathan.txt");
//     let expected = format!("{}/{}", full_path, "expected/jonathan.txt");

//     nu!(_output, cwd(&full_path), "mv jonathan.txt expected");

//     assert!(!h::file_exists_at(PathBuf::from(original_dir)));
//     assert!(h::file_exists_at(PathBuf::from(expected)));
// }

// #[test]
// fn moves_the_directory_inside_directory_if_path_to_move_is_existing_directory() {
//     let sandbox = Playground::setup_for("mv_test_5")
//         .within("contributors")
//         .with_files(vec![EmptyFile("jonathan.txt")])
//         .mkdir("expected")
//         .test_dir_name();

//     let full_path = format!("{}/{}", Playground::root(), sandbox);
//     let original_dir = format!("{}/{}", full_path, "contributors");
//     let expected = format!("{}/{}", full_path, "expected/contributors");

//     nu!(_output, cwd(&full_path), "mv contributors expected");

//     assert!(!h::dir_exists_at(PathBuf::from(original_dir)));
//     assert!(h::file_exists_at(PathBuf::from(expected)));
// }

// #[test]
// fn moves_the_directory_inside_directory_if_path_to_move_is_nonexistent_directory() {
//     let sandbox = Playground::setup_for("mv_test_6")
//         .within("contributors")
//         .with_files(vec![EmptyFile("jonathan.txt")])
//         .mkdir("expected")
//         .test_dir_name();

//     let full_path = format!("{}/{}", Playground::root(), sandbox);
//     let original_dir = format!("{}/{}", full_path, "contributors");

//     nu!(
//         _output,
//         cwd(&full_path),
//         "mv contributors expected/this_dir_exists_now/los_tres_amigos"
//     );

//     let expected = format!(
//         "{}/{}",
//         full_path, "expected/this_dir_exists_now/los_tres_amigos"
//     );

//     assert!(!h::dir_exists_at(PathBuf::from(original_dir)));
//     assert!(h::file_exists_at(PathBuf::from(expected)));
// }

// #[test]
// fn moves_using_path_with_wildcard() {
//     let sandbox = Playground::setup_for("mv_test_7")
//         .within("originals")
//         .with_files(vec![
//             EmptyFile("andres.ini"),
//             EmptyFile("caco3_plastics.csv"),
//             EmptyFile("cargo_sample.toml"),
//             EmptyFile("jonathan.ini"),
//             EmptyFile("jonathan.xml"),
//             EmptyFile("sgml_description.json"),
//             EmptyFile("sample.ini"),
//             EmptyFile("utf16.ini"),
//             EmptyFile("yehuda.ini"),
//         ])
//         .mkdir("work_dir")
//         .mkdir("expected")
//         .test_dir_name();

//     let full_path = format!("{}/{}", Playground::root(), sandbox);
//     let work_dir = format!("{}/{}", full_path, "work_dir");
//     let expected_copies_path = format!("{}/{}", full_path, "expected");

//     nu!(_output, cwd(&work_dir), "mv ../originals/*.ini ../expected");

//     assert!(h::files_exist_at(
//         vec![
//             Path::new("yehuda.ini"),
//             Path::new("jonathan.ini"),
//             Path::new("sample.ini"),
//             Path::new("andres.ini"),
//         ],
//         PathBuf::from(&expected_copies_path)
//     ));
// }

// #[test]
// fn moves_using_a_glob() {
//     let sandbox = Playground::setup_for("mv_test_8")
//         .within("meals")
//         .with_files(vec![
//             EmptyFile("arepa.txt"),
//             EmptyFile("empanada.txt"),
//             EmptyFile("taquiza.txt"),
//         ])
//         .mkdir("work_dir")
//         .mkdir("expected")
//         .test_dir_name();

//     let full_path = format!("{}/{}", Playground::root(), sandbox);
//     let meal_dir = format!("{}/{}", full_path, "meals");
//     let work_dir = format!("{}/{}", full_path, "work_dir");
//     let expected_copies_path = format!("{}/{}", full_path, "expected");

//     nu!(_output, cwd(&work_dir), "mv ../meals/* ../expected");

//     assert!(h::dir_exists_at(PathBuf::from(meal_dir)));
//     assert!(h::files_exist_at(
//         vec![
//             Path::new("arepa.txt"),
//             Path::new("empanada.txt"),
//             Path::new("taquiza.txt"),
//         ],
//         PathBuf::from(&expected_copies_path)
//     ));
// }
