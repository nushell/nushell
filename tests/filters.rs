// use test_support::{nu, pipeline};
// use test_support::playground::Playground;
// use test_support::fs::Stub::FileWithContentToBeTrimmed;

// #[test]
// fn can_sum() {
//     let actual = nu!(
//         cwd: "tests/fixtures/formats", h::pipeline(
//         r#"
//             open sgml_description.json
//             | get glossary.GlossDiv.GlossList.GlossEntry.Sections
//             | sum
//             | echo $it
//         "#
//     ));

//     assert_eq!(actual, "203")
// }

// #[test]
// fn can_average_numbers() {
//     let actual = nu!(
//         cwd: "tests/fixtures/formats", h::pipeline(
//         r#"
//             open sgml_description.json
//             | get glossary.GlossDiv.GlossList.GlossEntry.Sections
//             | average
//             | echo $it
//         "#
//     ));

//     assert_eq!(actual, "101.5000000000000")
// }

// #[test]
// fn can_average_bytes() {
//     let actual = nu!(
//         cwd: "tests/fixtures/formats",
//         "ls | sort-by name | skip 1 | first 2 | get size | average | echo $it"
//     );

//     assert_eq!(actual, "1600.000000000000");
// }
