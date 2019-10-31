mod helpers;

use helpers as h;
use helpers::{Playground, Stub::*};

use std::path::PathBuf;

#[test]
fn has_default_configuration_file() {
    let expected = "config.toml";

    Playground::setup("config_test_1", |dirs, _| {
        nu!(cwd: dirs.root(), "config");

        assert_eq!(
            dirs.config_path().join(expected),
            nu::config_path().unwrap().join(expected)
        );
    })
}

#[test]
fn shows_path_of_configuration_file() {
    let expected = "config.toml";

    Playground::setup("config_test_2", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "config --path | echo $it"
        );

        assert_eq!(PathBuf::from(actual), dirs.config_path().join(expected));
    });
}

#[test]
fn use_different_configuration() {
    Playground::setup("config_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "test_3.toml",
            r#"
                    caballero_1 = "Andrés N. Robalino"
                    caballero_2 = "Jonathan Turner"
                    caballero_3 = "Yehuda katz"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.root(),
            "config --get caballero_1 --load {}/test_3.toml | echo $it",
            dirs.test()
        );

        assert_eq!(actual, "Andrés N. Robalino");
    });

    h::delete_file_at(nu::config_path().unwrap().join("test_3.toml"));
}

#[test]
fn sets_configuration_value() {
    Playground::setup("config_test_4", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "test_4.toml",
            r#"
                    caballero_1 = "Andrés N. Robalino"
                    caballero_2 = "Jonathan Turner"
                    caballero_3 = "Yehuda katz"
                "#,
        )]);

        nu!(
            cwd: dirs.test(),
            "config --load test_4.toml --set [caballero_4 jonas]"
        );

        let actual = nu!(
            cwd: dirs.root(),
            r#"open "{}/test_4.toml" | get caballero_4 | echo $it"#,
            dirs.config_path()
        );

        assert_eq!(actual, "jonas");
    });

    h::delete_file_at(nu::config_path().unwrap().join("test_4.toml"));
}

// #[test]
// fn removes_configuration_value() {
//     Playground::setup("config_test_5", |dirs, sandbox| {
//         sandbox.with_files(vec![FileWithContent(
//             "test_5.toml",
//             r#"
//                     caballeros = [1, 1, 1]
//                     podershell = [1, 1, 1]
//                 "#,
//         )]);

//         nu!(
//             cwd: dirs.test(),
//             "config --load test_5.toml --remove podershell"
//         );

//         let actual = nu_error!(
//             cwd: dirs.root(),
//             r#"open "{}/test_5.toml" | get podershell | echo $it"#,
//             dirs.config_path()
//         );

//         assert!(actual.contains("Unknown column"));
//     });

//     h::delete_file_at(nu::config_path().unwrap().join("test_5.toml"));
// }
