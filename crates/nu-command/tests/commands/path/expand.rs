use nu_path::Path;
use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn expands_path_with_dot() {
    Playground::setup("path_expand_1", |dirs, sandbox| {
        sandbox.within("menu").with_files(&[EmptyFile("spam.txt")]);

        let actual = nu!(cwd: dirs.test(), r#"
            echo "menu/./spam.txt"
            | path expand
        "#);

        let expected = dirs.test.join("menu").join("spam.txt");
        assert_eq!(Path::new(&actual.out), expected);
    })
}

#[cfg(unix)]
#[test]
fn expands_path_without_follow_symlink() {
    Playground::setup("path_expand_3", |dirs, sandbox| {
        sandbox.within("menu").with_files(&[EmptyFile("spam.txt")]);

        let actual = nu!(cwd: dirs.test(), r#"
            ln -s spam.txt menu/spam_link.ln;
            echo "menu/./spam_link.ln"
            | path expand -n
        "#);

        let expected = dirs.test.join("menu").join("spam_link.ln");
        assert_eq!(Path::new(&actual.out), expected);
    })
}

#[test]
fn expands_path_with_double_dot() {
    Playground::setup("path_expand_2", |dirs, sandbox| {
        sandbox.within("menu").with_files(&[EmptyFile("spam.txt")]);

        let actual = nu!(cwd: dirs.test(), r#"
            echo "menu/../menu/spam.txt"
            | path expand
        "#);

        let expected = dirs.test.join("menu").join("spam.txt");
        assert_eq!(Path::new(&actual.out), expected);
    })
}

#[test]
fn const_path_expand() {
    Playground::setup("const_path_expand", |dirs, sandbox| {
        sandbox.within("menu").with_files(&[EmptyFile("spam.txt")]);

        let actual = nu!(cwd: dirs.test(), r#"
            const result = ("menu/./spam.txt" | path expand);
            $result
        "#);

        let expected = dirs.test.join("menu").join("spam.txt");
        assert_eq!(Path::new(&actual.out), expected);
    })
}

#[cfg(windows)]
mod windows {
    use super::*;

    #[test]
    fn expands_path_with_tilde_backward_slash() {
        Playground::setup("path_expand_2", |dirs, _| {
            let actual = nu!(cwd: dirs.test(), r#"
                echo "~\tmp.txt" | path expand
            "#);

            assert!(!Path::new(&actual.out).starts_with("~"));
        })
    }

    #[test]
    fn win_expands_path_with_tilde_forward_slash() {
        Playground::setup("path_expand_2", |dirs, _| {
            let actual = nu!(cwd: dirs.test(), r#"
                echo "~/tmp.txt" | path expand
            "#);

            assert!(!Path::new(&actual.out).starts_with("~"));
        })
    }

    #[test]
    fn expands_path_without_follow_symlink() {
        Playground::setup("path_expand_3", |dirs, sandbox| {
            sandbox.within("menu").with_files(&[EmptyFile("spam.txt")]);

            let cwd = dirs.test();
            std::os::windows::fs::symlink_file(
                cwd.join("menu").join("spam.txt"),
                cwd.join("menu").join("spam_link.ln"),
            )
            .unwrap();

            let actual = nu!(cwd: dirs.test(), r#"
            echo "menu/./spam_link.ln"
            | path expand -n
                        "#);

            let expected = dirs.test.join("menu").join("spam_link.ln");
            assert_eq!(Path::new(&actual.out), expected);
        })
    }
}
