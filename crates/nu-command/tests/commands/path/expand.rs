use nu_path::Path;
use nu_test_support::prelude::*;

#[test]
fn expands_path_with_dot(playground: Playground) -> Result {
    playground.empty_file("menu/spam.txt")?;

    let code = r#"
        echo "menu/./spam.txt"
        | path expand
    "#;

    let outcome: String = test().cwd(playground.path()).run(code)?;
    let expected = playground.path().join("menu").join("spam.txt");
    assert_eq!(Path::new(&outcome), expected);
    Ok(())
}

#[cfg(unix)]
#[test]
fn expands_path_without_follow_symlink(playground: Playground) -> Result {
    playground.empty_file("menu/spam.txt")?;

    let code = r#"
        ln -s spam.txt menu/spam_link.ln;
        echo "menu/./spam_link.ln"
        | path expand -n
    "#;

    let outcome: String = test().inherit_path().cwd(playground.path()).run(code)?;
    let expected = playground.path().join("menu").join("spam_link.ln");
    assert_eq!(Path::new(&outcome), expected);
    Ok(())
}

#[test]
fn expands_path_with_double_dot(playground: Playground) -> Result {
    playground.empty_file("menu/spam.txt")?;

    let code = r#"
        echo "menu/../menu/spam.txt"
        | path expand
    "#;

    let outcome: String = test().cwd(playground.path()).run(code)?;
    let expected = playground.path().join("menu").join("spam.txt");
    assert_eq!(Path::new(&outcome), expected);
    Ok(())
}

#[test]
fn const_path_expand(playground: Playground) -> Result {
    playground.empty_file("menu/spam.txt")?;

    let code = r#"
        const result = ("menu/./spam.txt" | path expand);
        $result
    "#;

    let outcome: String = test().cwd(playground.path()).run(code)?;
    let expected = playground.path().join("menu").join("spam.txt");
    assert_eq!(Path::new(&outcome), expected);
    Ok(())
}

#[cfg(windows)]
mod windows {
    use super::*;

    #[test]
    fn expands_path_with_tilde_backward_slash(playground: Playground) -> Result {
        let code = r#"
            echo "~\tmp.txt" | path expand
        "#;

        let outcome: String = test().cwd(playground.path()).run(code)?;
        assert!(!Path::new(&outcome).starts_with("~"));
        Ok(())
    }

    #[test]
    fn win_expands_path_with_tilde_forward_slash(playground: Playground) -> Result {
        let code = r#"
            echo "~/tmp.txt" | path expand
        "#;

        let outcome: String = test().cwd(playground.path()).run(code)?;
        assert!(!Path::new(&outcome).starts_with("~"));
        Ok(())
    }

    #[test]
    fn expands_path_without_follow_symlink(playground: Playground) -> Result {
        playground.empty_file("menu/spam.txt")?;
        playground.symlink("menu/spam.txt", "menu/spam_link.ln")?;

        let code = r#"
            echo "menu/./spam_link.ln"
            | path expand -n
        "#;

        let outcome: String = test().cwd(playground.path()).run(code)?;
        let expected = playground.path().join("menu").join("spam_link.ln");
        assert_eq!(Path::new(&outcome), expected);
        Ok(())
    }
}
