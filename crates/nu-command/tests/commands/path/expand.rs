use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

use std::path::PathBuf;

#[test]
fn expands_path_with_dot() {
    Playground::setup("path_expand_1", |dirs, sandbox| {
        sandbox
            .within("menu")
            .with_files(vec![EmptyFile("spam.txt")]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                echo "menu/./spam.txt"
                | path expand
            "#
        ));

        let expected = dirs.test.join("menu").join("spam.txt");
        assert_eq!(PathBuf::from(actual.out), expected);
    })
}

#[cfg(unix)]
#[test]
fn expands_path_without_follow_symlink() {
    Playground::setup("path_expand_3", |dirs, sandbox| {
        sandbox
            .within("menu")
            .with_files(vec![EmptyFile("spam.txt")]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ln -s spam.txt menu/spam_link.ln;
                echo "menu/./spam_link.ln"
                | path expand -n
            "#
        ));

        let expected = dirs.test.join("menu").join("spam_link.ln");
        assert_eq!(PathBuf::from(actual.out), expected);
    })
}

#[test]
fn expands_path_with_double_dot() {
    Playground::setup("path_expand_2", |dirs, sandbox| {
        sandbox
            .within("menu")
            .with_files(vec![EmptyFile("spam.txt")]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                echo "menu/../menu/spam.txt"
                | path expand
            "#
        ));

        let expected = dirs.test.join("menu").join("spam.txt");
        assert_eq!(PathBuf::from(actual.out), expected);
    })
}

#[cfg(windows)]
mod windows {
    use super::*;

    #[test]
    fn expands_path_with_tilde_backward_slash() {
        Playground::setup("path_expand_2", |dirs, _| {
            let actual = nu!(
                cwd: dirs.test(), pipeline(
                r#"
                    echo "~\tmp.txt" | path expand
                "#
            ));

            assert!(!PathBuf::from(actual.out).starts_with("~"));
        })
    }

    #[test]
    fn win_expands_path_with_tilde_forward_slash() {
        Playground::setup("path_expand_2", |dirs, _| {
            let actual = nu!(
                cwd: dirs.test(), pipeline(
                r#"
                    echo "~/tmp.txt" | path expand
                "#
            ));

            assert!(!PathBuf::from(actual.out).starts_with("~"));
        })
    }

    #[test]
    fn expands_path_without_follow_symlink() {
        Playground::setup("path_expand_3", |dirs, sandbox| {
            sandbox
                .within("menu")
                .with_files(vec![EmptyFile("spam.txt")]);

            let cwd = dirs.test();
            std::os::windows::fs::symlink_file(
                cwd.join("menu").join("spam.txt"),
                cwd.join("menu").join("spam_link.ln"),
            )
            .unwrap();

            let actual = nu!(
                cwd: dirs.test(), pipeline(
                r#"
                echo "menu/./spam_link.ln"
                | path expand -n
            "#
            ));

            let expected = dirs.test.join("menu").join("spam_link.ln");
            assert_eq!(PathBuf::from(actual.out), expected);
        })
    }
}
