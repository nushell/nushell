// use nu_test_support::fs::{file_contents, Stub};
use nu_test_support::nu;
use nu_test_support::playground::Playground;

// #[cfg(windows)]
// #[test]
// fn wait_pwd_per_drive() {
//     Playground::setup("save_test_pwd_per_drive", |dirs, sandbox| {
//         sandbox.mkdir("test_folder");
//         let _actual = nu!(
//             cwd: dirs.test(),
//             r#"
//                 subst X: /D | touch out
//                 subst X: test_folder
//                 x:
//                 mkdir test_folder_on_x
//                 cd -
//                 watch x:test_folder_on_x { |op, path| $"($op) - ($path)(char nl)" | save --append changes_in_test_folder_on_x.log }
//                 touch x:test_folder_on_x\test_file_on_x.txt
//                 sleep 3000ms
//                 subst X: /D | touch out
//             "#
//         );
//         assert_eq!(_actual.out, r"x");
//         assert!(_actual.err.is_empty());
//     })
// }
