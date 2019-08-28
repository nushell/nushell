mod helpers;

use h::{in_directory as cwd, Playground, Stub::*};
use helpers as h;
use std::path::{Path, PathBuf};

// #[test]
// fn rm_removes_a_file() {
//     let sandbox = Playground::setup_for("rm_regular_file_test")
//         .with_files(vec![EmptyFile("i_will_be_deleted.txt")])
//         .test_dir_name();

//     nu!(
//         _output,
//         cwd(&Playground::root()),
//         "rm rm_regular_file_test/i_will_be_deleted.txt"
//     );

//     let path = &format!(
//         "{}/{}/{}",
//         Playground::root(),
//         sandbox,
//         "i_will_be_deleted.txt"
//     );

//     assert!(!h::file_exists_at(PathBuf::from(path)));
// }

// #[test]
// fn rm_removes_files_with_wildcard() {
//     let sandbox = Playground::setup_for("rm_wildcard_test_1")
//         .within("src")
//         .with_files(vec![
//             EmptyFile("cli.rs"),
//             EmptyFile("lib.rs"),
//             EmptyFile("prelude.rs"),
//         ])
//         .within("src/parser")
//         .with_files(vec![EmptyFile("parse.rs"), EmptyFile("parser.rs")])
//         .within("src/parser/parse")
//         .with_files(vec![EmptyFile("token_tree.rs")])
//         .within("src/parser/hir")
//         .with_files(vec![
//             EmptyFile("baseline_parse.rs"),
//             EmptyFile("baseline_parse_tokens.rs"),
//         ])
//         .test_dir_name();

//     let full_path = format!("{}/{}", Playground::root(), sandbox);

//     nu!(
//         _output,
//         cwd("tests/fixtures/nuplayground/rm_wildcard_test_1"),
//         r#"rm "src/*/*/*.rs""#
//     );

//     assert!(!h::files_exist_at(
//         vec![
//             Path::new("src/parser/parse/token_tree.rs"),
//             Path::new("src/parser/hir/baseline_parse.rs"),
//             Path::new("src/parser/hir/baseline_parse_tokens.rs")
//         ],
//         PathBuf::from(&full_path)
//     ));

//     assert_eq!(
//         Playground::glob_vec(&format!("{}/src/*/*/*.rs", &full_path)),
//         Vec::<PathBuf>::new()
//     );
// }

// #[test]
// fn rm_removes_deeply_nested_directories_with_wildcard_and_recursive_flag() {
//     let sandbox = Playground::setup_for("rm_wildcard_test_2")
//         .within("src")
//         .with_files(vec![
//             EmptyFile("cli.rs"),
//             EmptyFile("lib.rs"),
//             EmptyFile("prelude.rs"),
//         ])
//         .within("src/parser")
//         .with_files(vec![EmptyFile("parse.rs"), EmptyFile("parser.rs")])
//         .within("src/parser/parse")
//         .with_files(vec![EmptyFile("token_tree.rs")])
//         .within("src/parser/hir")
//         .with_files(vec![
//             EmptyFile("baseline_parse.rs"),
//             EmptyFile("baseline_parse_tokens.rs"),
//         ])
//         .test_dir_name();

//     let full_path = format!("{}/{}", Playground::root(), sandbox);

//     nu!(
//         _output,
//         cwd("tests/fixtures/nuplayground/rm_wildcard_test_2"),
//         "rm src/* --recursive"
//     );

//     assert!(!h::files_exist_at(
//         vec![Path::new("src/parser/parse"), Path::new("src/parser/hir"),],
//         PathBuf::from(&full_path)
//     ));
// }

// #[test]
// fn rm_removes_directory_contents_without_recursive_flag_if_empty() {
//     let sandbox = Playground::setup_for("rm_directory_removal_recursively_test_1").test_dir_name();

//     nu!(
//         _output,
//         cwd("tests/fixtures/nuplayground"),
//         "rm rm_directory_removal_recursively_test_1"
//     );

//     let expected = format!("{}/{}", Playground::root(), sandbox);

//     assert!(!h::file_exists_at(PathBuf::from(expected)));
// }

// #[test]
// fn rm_removes_directory_contents_with_recursive_flag() {
//     let sandbox = Playground::setup_for("rm_directory_removal_recursively_test_2")
//         .with_files(vec![
//             EmptyFile("yehuda.txt"),
//             EmptyFile("jonathan.txt"),
//             EmptyFile("andres.txt"),
//         ])
//         .test_dir_name();

//     nu!(
//         _output,
//         cwd("tests/fixtures/nuplayground"),
//         "rm rm_directory_removal_recursively_test_2 --recursive"
//     );

//     let expected = format!("{}/{}", Playground::root(), sandbox);

//     assert!(!h::file_exists_at(PathBuf::from(expected)));
// }

// #[test]
// fn rm_errors_if_attempting_to_delete_a_directory_with_content_without_recursive_flag() {
//     let sandbox = Playground::setup_for("rm_prevent_directory_removal_without_flag_test")
//         .with_files(vec![EmptyFile("some_empty_file.txt")])
//         .test_dir_name();

//     let full_path = format!("{}/{}", Playground::root(), sandbox);

//     nu_error!(
//         output,
//         cwd(&Playground::root()),
//         "rm rm_prevent_directory_removal_without_flag_test"
//     );

//     assert!(h::file_exists_at(PathBuf::from(full_path)));
//     assert!(output.contains("is a directory"));
// }

// #[test]
// fn rm_errors_if_attempting_to_delete_single_dot_as_argument() {
//     nu_error!(output, cwd(&Playground::root()), "rm .");

//     assert!(output.contains("may not be removed"));
// }

// #[test]
// fn rm_errors_if_attempting_to_delete_two_dot_as_argument() {
//     nu_error!(output, cwd(&Playground::root()), "rm ..");

//     assert!(output.contains("may not be removed"));
// }
