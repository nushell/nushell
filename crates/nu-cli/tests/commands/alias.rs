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
fn error_alias_wrong_shape_shallow() {
    let actual = nu!(
        cwd: ".",
        r#"
        alias round-to [num digits] { echo $num | str from -d $digits }
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
        alias round-to [nums digits] { echo $nums | each {= $(str from -d $digits)}}
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
        alias round-plus-one [nums digits] { echo $nums | each {= $(str from -d $digits | str to-decimal) + 1}}
        round-plus-one 3.45 a
        "#
    );

    assert!(actual.err.contains("Type"));
}

// TODO make work? (if binary is always a, a -> a)
#[test]
fn error_alias_wrong_shape_deeper_binary() {
    let actual = nu!(
        cwd: ".",
        r#"
        alias round-one-more [num digits] { echo $num | str from -d $(= $digits + 1) }
        round-one-more 3.45 a
        "#
    );

    assert!(actual.err.contains("Type"));
}
