use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn basic_exec() {
    Playground::setup("test_exec_1", |dirs, _| {
        let actual = nu!(cwd: dirs.test(), r#"
            nu -n -c 'exec nu --testbin cococo a b c'
        "#);

        assert_eq!(actual.out, "a b c");
    })
}

#[test]
fn exec_complex_args() {
    Playground::setup("test_exec_2", |dirs, _| {
        let actual = nu!(cwd: dirs.test(), r#"
            nu -n -c 'exec nu --testbin cococo b --bar=2 -sab --arwr - -DTEEE=aasd-290 -90 --'
        "#);

        assert_eq!(actual.out, "b --bar=2 -sab --arwr - -DTEEE=aasd-290 -90 --");
    })
}

#[test]
fn exec_fail_batched_short_args() {
    Playground::setup("test_exec_3", |dirs, _| {
        let actual = nu!(cwd: dirs.test(), r#"
            nu -n -c 'exec nu --testbin cococo -ab 10'
        "#);

        assert_eq!(actual.out, "");
    })
}

#[test]
fn exec_misc_values() {
    Playground::setup("test_exec_4", |dirs, _| {
        let actual = nu!(cwd: dirs.test(), r#"
            nu -n -c 'let x = "abc"; exec nu --testbin cococo $x ...[ a b c ]'
        "#);

        assert_eq!(actual.out, "abc a b c");
    })
}
