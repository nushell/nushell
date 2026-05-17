use nu_path::Path;
use nu_test_support::{fs::Stub::EmptyFile, prelude::*};

#[test]
fn expands_path_with_dot() -> Result {
    Playground::setup("path_expand_1", |dirs, sandbox| {
        sandbox.within("menu").with_files(&[EmptyFile("spam.txt")]);

        let code = r#"
            echo "menu/./spam.txt"
            | path expand
        "#;

        let outcome: String = test().cwd(dirs.test()).run(code)?;
        let expected = dirs.test.join("menu").join("spam.txt");
        assert_eq!(Path::new(&outcome), expected);
        Ok(())
    })
}

#[cfg(unix)]
#[test]
fn expands_path_without_follow_symlink() -> Result {
    Playground::setup("path_expand_3", |dirs, sandbox| {
        sandbox.within("menu").with_files(&[EmptyFile("spam.txt")]);

        let code = r#"
            ln -s spam.txt menu/spam_link.ln;
            echo "menu/./spam_link.ln"
            | path expand -n
        "#;

        let outcome: String = test().inherit_path().cwd(dirs.test()).run(code)?;
        let expected = dirs.test.join("menu").join("spam_link.ln");
        assert_eq!(Path::new(&outcome), expected);
        Ok(())
    })
}

#[test]
fn expands_path_with_double_dot() -> Result {
    Playground::setup("path_expand_2", |dirs, sandbox| {
        sandbox.within("menu").with_files(&[EmptyFile("spam.txt")]);

        let code = r#"
            echo "menu/../menu/spam.txt"
            | path expand
        "#;

        let outcome: String = test().cwd(dirs.test()).run(code)?;
        let expected = dirs.test.join("menu").join("spam.txt");
        assert_eq!(Path::new(&outcome), expected);
        Ok(())
    })
}

#[test]
fn const_path_expand() -> Result {
    Playground::setup("const_path_expand", |dirs, sandbox| {
        sandbox.within("menu").with_files(&[EmptyFile("spam.txt")]);

        let code = r#"
            const result = ("menu/./spam.txt" | path expand);
            $result
        "#;

        let outcome: String = test().cwd(dirs.test()).run(code)?;
        let expected = dirs.test.join("menu").join("spam.txt");
        assert_eq!(Path::new(&outcome), expected);
        Ok(())
    })
}

#[cfg(windows)]
mod windows {
    use super::*;

    #[test]
    fn expands_path_with_tilde_backward_slash() -> Result {
        Playground::setup("path_expand_2", |dirs, _| {
            let code = r#"
                echo "~\tmp.txt" | path expand
            "#;

            let outcome: String = test().cwd(dirs.test()).run(code)?;
            assert!(!Path::new(&outcome).starts_with("~"));
            Ok(())
        })
    }

    #[test]
    fn win_expands_path_with_tilde_forward_slash() -> Result {
        Playground::setup("path_expand_2", |dirs, _| {
            let code = r#"
                echo "~/tmp.txt" | path expand
            "#;

            let outcome: String = test().cwd(dirs.test()).run(code)?;
            assert!(!Path::new(&outcome).starts_with("~"));
            Ok(())
        })
    }

    #[test]
    fn expands_path_without_follow_symlink() -> Result {
        Playground::setup("path_expand_3", |dirs, sandbox| {
            sandbox.within("menu").with_files(&[EmptyFile("spam.txt")]);

            let cwd = dirs.test();
            std::os::windows::fs::symlink_file(
                cwd.join("menu").join("spam.txt"),
                cwd.join("menu").join("spam_link.ln"),
            )
            .unwrap();

            let code = r#"
            echo "menu/./spam_link.ln"
            | path expand -n
                        "#;

            let outcome: String = test().cwd(dirs.test()).run(code)?;
            let expected = dirs.test.join("menu").join("spam_link.ln");
            assert_eq!(Path::new(&outcome), expected);
            Ok(())
        })
    }
}
