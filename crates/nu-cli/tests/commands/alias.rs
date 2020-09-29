use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn alias_args_work() {
    Playground::setup("append_test_1", |dirs, _| {
        let actual = nu!(
            cwd: dirs.root(),
            r#"
                alias double_echo [a b] {echo $a $b}
                double_echo 1 2 | to json
            "#
        );

        assert_eq!(actual.out, "[1,2]");
    })
}

#[test]
#[cfg(not(windows))]
fn alias_parses_path_tilde() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        r#"
        alias -i new-cd [dir] { cd $dir }
        new-cd ~
        pwd
        "#
    );

    #[cfg(target_os = "linux")]
    assert!(actual.out.contains("home"));
    #[cfg(target_os = "macos")]
    assert!(actual.out.contains("Users"));
}

#[test]
fn error_alias_wrong_shape_shallow() {
    let actual = nu!(
        cwd: ".",
        r#"
        alias -i round-to [num digits] { echo $num | str from -d $digits }
        round-to 3.45 a
        "#
    );

    assert!(actual.err.contains("Type"));
}

#[test]
fn error_alias_wrong_shape_deep_invocation() {
    let actual = nu!(
        cwd: ".",
        r#"
        alias -i round-to [nums digits] { echo $nums | each {= $(str from -d $digits)}}
        round-to 3.45 a
        "#
    );

    assert!(actual.err.contains("Type"));
}

#[test]
fn error_alias_wrong_shape_deep_binary() {
    let actual = nu!(
        cwd: ".",
        r#"
        alias -i round-plus-one [nums digits] { echo $nums | each {= $(str from -d $digits | str to-decimal) + 1}}
        round-plus-one 3.45 a
        "#
    );

    assert!(actual.err.contains("Type"));
}

#[test]
fn error_alias_wrong_shape_deeper_binary() {
    let actual = nu!(
        cwd: ".",
        r#"
        alias -i round-one-more [num digits] { echo $num | str from -d $(= $digits + 1) }
        round-one-more 3.45 a
        "#
    );

    assert!(actual.err.contains("Type"));
}

#[test]
fn error_alias_syntax_shape_clash() {
    let actual = nu!(
        cwd: ".",
        r#"
        alias -i clash [a] { echo 1.1 2 3 | each { str from -d $a } | range $a } }
        "#
    );

    assert!(actual.err.contains("alias"));
}
