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
