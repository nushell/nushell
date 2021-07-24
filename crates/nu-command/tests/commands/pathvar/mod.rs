use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::fs::{AbsolutePath, DisplayPath};
use nu_test_support::playground::{says, Playground};
use nu_test_support::{nu, NATIVE_PATH_ENV_SEPARATOR};

use std::path::PathBuf;

use hamcrest2::assert_that;
use hamcrest2::prelude::*;

/// Helper function that joins string literals with ':' or ';', based on host OS
fn join_env_sep(pieces: &[&str]) -> String {
    let sep_string = String::from(NATIVE_PATH_ENV_SEPARATOR);
    pieces.join(&sep_string)
}

// Helpers

#[cfg(windows)]
#[test]
fn joins_env_on_windows() {
    let pieces = ["sausage", "bacon", "spam"];
    let actual = join_env_sep(&pieces);

    assert_eq!(&actual, "sausage;bacon;spam");
}

#[cfg(not(windows))]
#[test]
fn joins_env_on_non_windows() {
    let pieces = ["sausage", "bacon", "spam"];
    let actual = join_env_sep(&pieces);

    assert_eq!(&actual, "sausage:bacon:spam");
}

// pathvar

// The following test doesn't work likely because of this issue:
//   https://github.com/nushell/nushell/issues/3831
// #[test]
// fn pathvar_correctly_reads_path_from_config_and_env() {
//     Playground::setup("hi_there", |dirs, sandbox| {
//         let file = AbsolutePath::new(dirs.test().join("config.toml"));
//
//         sandbox
//             .with_files(vec![FileWithContent(
//                 "config.toml",
//                 r#"
//                     skip_welcome_message = true
//
//                     path = ["/Users/andresrobalino/.volta/bin", "/Users/mosqueteros/bin"]
//                 "#,
//             )])
//             .with_config(&file)
//             .with_env(
//                 nu_test_support::NATIVE_PATH_ENV_VAR,
//                 &PathBuf::from("/Users/mosquito/proboscis").display_path(),
//             );
//
//         let expected =
//             "/Users/andresrobalino/.volta/bin-/Users/mosqueteros/bin-/Users/mosquito/proboscis";
//         let actual = sandbox.pipeline(r#" pathvar | str collect '-' "#);
//
//         assert_that!(actual, says().stdout(&expected));
//     })
// }

#[test]
fn pathvar_correctly_reads_path_from_config() {
    Playground::setup("hi_there", |dirs, sandbox| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        sandbox
            .with_files(vec![FileWithContent(
                "config.toml",
                r#"
                    skip_welcome_message = true

                    path = ["/Users/andresrobalino/.volta/bin", "/Users/mosqueteros/bin"]
                "#,
            )])
            .with_config(&file)
            .with_env(
                nu_test_support::NATIVE_PATH_ENV_VAR,
                &PathBuf::from("/Users/mosquito/proboscis").display_path(),
            );

        let expected = "/Users/andresrobalino/.volta/bin-/Users/mosqueteros/bin";
        let actual = sandbox.pipeline(r#" pathvar | str collect '-' "#);

        assert_that!(actual, says().stdout(&expected));
    })
}

// The following test doesn't work likely because of this issue:
//   https://github.com/nushell/nushell/issues/3831
// #[test]
// fn pathvar_correctly_reads_path_from_env() {
//     Playground::setup("hi_there", |_, sandbox| {
//         sandbox
//             .with_env(
//                 nu_test_support::NATIVE_PATH_ENV_VAR,
//                 &PathBuf::from("/Users/mosquito/proboscis").display_path(),
//             );
//
//         let expected = "/Users/mosquito/proboscis";
//         let actual = sandbox.pipeline(r#" pathvar | str collect '-' "#);
//
//         assert_that!(actual, says().stdout(&expected));
//     })
// }

// Doesn't work because Nushell is not set up to read other vars than path from config
// Maybe also https://github.com/nushell/nushell/issues/3831
// #[test]
// fn pathvar_correctly_reads_env_var_from_config_and_env() {
//     Playground::setup("hi_there", |dirs, sandbox| {
//         let file = AbsolutePath::new(dirs.test().join("config.toml"));
//
//         sandbox
//             .with_files(vec![FileWithContent(
//                 "config.toml",
//                 r#"
//                     skip_welcome_message = true
//
//                     breakfast = ["egg", "sausage"]
//                 "#,
//             )])
//             .with_config(&file)
//             .with_env(
//                 "BREAKFAST",
//                 &join_env_sep(&["bacon", "spam"]),
//             );
//
//         let expected = "egg-sausage-bacon-spam";
//         let actual = sandbox.pipeline(r#" pathvar -v BREAKFAST | str collect '-' "#);
//
//         assert_that!(actual, says().stdout(&expected));
//     })
// }

// Doesn't work because Nushell is not set up to read other vars than path from config
// #[test]
// fn pathvar_correctly_reads_env_var_from_config() {
//     Playground::setup("hi_there", |dirs, sandbox| {
//         let file = AbsolutePath::new(dirs.test().join("config.toml"));
//
//         sandbox
//             .with_files(vec![FileWithContent(
//                 "config.toml",
//                 r#"
//                     skip_welcome_message = true
//
//                     breakfast = ["egg", "sausage"]
//                 "#,
//             )])
//             .with_config(&file);
//
//         let expected = "egg-sausage";
//         let actual = sandbox.pipeline(r#" pathvar -v BREAKFAST | str collect '-' "#);
//
//         assert_that!(actual, says().stdout(&expected));
//     })
// }

#[test]
fn pathvar_correctly_reads_env_var_from_env() {
    Playground::setup("hi_there", |_, sandbox| {
        sandbox.with_env("BREAKFAST", &join_env_sep(&["bacon", "spam"]));

        let expected = "bacon-spam";
        let actual = sandbox.pipeline(r#" pathvar -v BREAKFAST | str collect '-' "#);

        assert_that!(actual, says().stdout(&expected));
    })
}

// pathvar add

#[test]
fn pathvar_adds_to_path() {
    Playground::setup("hi_there", |dirs, sandbox| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        sandbox
            .with_files(vec![FileWithContent(
                "config.toml",
                r#"
                    skip_welcome_message = true

                    path = ["/Users/mosquito/proboscis"]
                "#,
            )])
            .with_config(&file);

        let expected = "spam-/Users/mosquito/proboscis";
        let actual = sandbox.pipeline(r#" pathvar add spam; pathvar | str collect '-' "#);

        assert_that!(actual, says().stdout(&expected));
    })
}

#[test]
fn pathvar_adds_to_env_var() {
    Playground::setup("hi_there", |_, sandbox| {
        sandbox.with_env("BREAKFAST", &join_env_sep(&["egg", "sausage", "bacon"]));

        let expected = join_env_sep(&["spam", "egg", "sausage", "bacon"]);
        let actual = sandbox.pipeline(
            r#" 
                pathvar add -v BREAKFAST spam
                $nu.env.BREAKFAST
            "#,
        );

        assert_that!(actual, says().stdout(&expected));
    })
}

// pathvar append

#[test]
fn pathvar_appends_to_path() {
    Playground::setup("hi_there", |dirs, sandbox| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        sandbox
            .with_files(vec![FileWithContent(
                "config.toml",
                r#"
                    skip_welcome_message = true

                    path = ["/Users/mosquito/proboscis"]
                "#,
            )])
            .with_config(&file);

        let expected = "/Users/mosquito/proboscis-spam";
        let actual = sandbox.pipeline(r#" pathvar append spam; pathvar | str collect '-' "#);

        assert_that!(actual, says().stdout(&expected));
    })
}

#[test]
fn pathvar_appends_to_env_var() {
    Playground::setup("hi_there", |_, sandbox| {
        sandbox.with_env("BREAKFAST", &join_env_sep(&["egg", "sausage", "bacon"]));

        let expected = join_env_sep(&["egg", "sausage", "bacon", "spam"]);
        let actual = sandbox.pipeline(
            r#" 
                pathvar append -v BREAKFAST spam
                $nu.env.BREAKFAST
            "#,
        );

        assert_that!(actual, says().stdout(&expected));
    })
}

// pathvar remove

#[test]
fn pathvar_removes_from_path() {
    Playground::setup("hi_there", |dirs, sandbox| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        sandbox
            .with_files(vec![FileWithContent(
                "config.toml",
                r#"
                    skip_welcome_message = true

                    path = ["/Users/mosquito/proboscis", "spam"]
                "#,
            )])
            .with_config(&file);

        let expected = "/Users/mosquito/proboscis";
        let actual = sandbox.pipeline(r#" pathvar remove 1; pathvar"#);

        assert_that!(actual, says().stdout(&expected));
    })
}

#[test]
fn pathvar_removes_from_env_var() {
    Playground::setup("hi_there", |_, sandbox| {
        sandbox.with_env(
            "BREAKFAST",
            &join_env_sep(&["egg", "sausage", "bacon", "spam"]),
        );

        let expected = join_env_sep(&["egg", "sausage", "bacon"]);
        let actual = sandbox.pipeline(
            r#" 
                pathvar remove -v BREAKFAST 3
                $nu.env.BREAKFAST
            "#,
        );

        assert_that!(actual, says().stdout(&expected));
    })
}

// pathvar reset

#[test]
fn pathvar_resets_path_from_config() {
    Playground::setup("hi_there", |dirs, sandbox| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        sandbox
            .with_files(vec![FileWithContent(
                "config.toml",
                r#"
                    skip_welcome_message = true

                    path = ["/Users/andresrobalino/.volta/bin", "/Users/mosqueteros/bin"]
                "#,
            )])
            .with_config(&file)
            .with_env(
                nu_test_support::NATIVE_PATH_ENV_VAR,
                &PathBuf::from("/Users/mosquito/proboscis").display_path(),
            );

        let expected = "/Users/andresrobalino/.volta/bin-/Users/mosqueteros/bin";
        let actual = sandbox.pipeline(
            r#"
                pathvar reset
                pathvar | str collect '-'
            "#,
        );

        assert_that!(actual, says().stdout(&expected));
    })
}

#[test]
fn pathvar_resets_env_var_from_config() {
    Playground::setup("hi_there", |dirs, sandbox| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        sandbox
            .with_files(vec![FileWithContent(
                "config.toml",
                r#"
                    skip_welcome_message = true

                    breakfast = ["egg", "sausage", "bacon"]
                "#,
            )])
            .with_config(&file)
            .with_env(
                "BREAKFAST",
                &join_env_sep(&["egg", "sausage", "bacon", "spam"]),
            );

        let expected = "egg-sausage-bacon";
        let actual = sandbox.pipeline(
            r#"
                pathvar reset -v BREAKFAST
                pathvar -v BREAKFAST | str collect '-'
            "#,
        );

        assert_that!(actual, says().stdout(&expected));
    })
}

// pathvar save

#[test]
fn pathvar_saves_path_to_config() {
    Playground::setup("hi_there", |dirs, sandbox| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        sandbox
            .with_files(vec![FileWithContent(
                "config.toml",
                r#"
                    skip_welcome_message = true

                    path = ["/Users/andresrobalino/.volta/bin", "/Users/mosqueteros/bin"]
                "#,
            )])
            .with_config(&file);

        let expected =
            "/Users/andresrobalino/.volta/bin-/Users/mosqueteros/bin-/Users/mosquito/proboscis";
        let actual = sandbox.pipeline(
            r#"
                pathvar append "/Users/mosquito/proboscis"
                pathvar save
                (config).path | str collect '-'
            "#,
        );

        assert_that!(actual, says().stdout(&expected));
    })
}

// The following test doesn't work likely because of this issue:
//   https://github.com/nushell/nushell/issues/3831
// #[test]
// fn pathvar_saves_new_path_to_config() {
//     Playground::setup("hi_there", |dirs, sandbox| {
//         let file = AbsolutePath::new(dirs.test().join("config.toml"));
//
//         sandbox
//             .with_files(vec![FileWithContent(
//                 "config.toml",
//                 r#"
//                     skip_welcome_message = true
//                 "#,
//             )])
//             .with_config(&file);
//
//         let expected = "/Users/mosquito/proboscis";
//         let actual = sandbox.pipeline(
//             r#"
//                 pathvar append "/Users/mosquito/proboscis"
//                 pathvar save
//                 (config).path | str collect '-'
//             "#,
//         );
//
//         assert_that!(actual, says().stdout(&expected));
//     })
// }

#[test]
fn pathvar_saves_env_var_to_config() {
    Playground::setup("hi_there", |dirs, sandbox| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        sandbox
            .with_files(vec![FileWithContent(
                "config.toml",
                r#"
                    skip_welcome_message = true

                    breakfast = ["egg", "sausage", "bacon"]
                "#,
            )])
            .with_config(&file)
            .with_env("BREAKFAST", "spam");

        let expected = "spam";
        let actual = sandbox.pipeline(
            r#"
                pathvar save -v BREAKFAST
                (config).breakfast | str collect '-'
            "#,
        );

        assert_that!(actual, says().stdout(&expected));
    })
}

#[test]
fn pathvar_saves_new_env_var_to_config() {
    Playground::setup("hi_there", |dirs, sandbox| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        sandbox
            .with_files(vec![FileWithContent(
                "config.toml",
                r#"
                    skip_welcome_message = true
                "#,
            )])
            .with_config(&file)
            .with_env("BREAKFAST", "spam");

        let expected = "spam";
        let actual = sandbox.pipeline(
            r#"
                pathvar save -v BREAKFAST
                (config).breakfast | str collect '-'
            "#,
        );

        assert_that!(actual, says().stdout(&expected));
    })
}

// test some errors

#[test]
fn pathvar_error_non_existent_env_var() {
    Playground::setup("hi_there", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "pathvar -v EGGS_BACON_SPAM_SAUSAGE_SPAM_AND_SPAM_WITH_EXTRA_SPAM"
        );

        assert!(actual.err.contains("Error"));
        assert!(actual.err.contains("not set"));
    })
}
