use nu_test_support::nu;
use nu_test_support::playground::Playground;
use std::fs;

#[test]
fn def_with_comment() {
    Playground::setup("def_with_comment", |dirs, _| {
        let data = r#"
#My echo
def e [arg] {echo $arg}
            "#;
        fs::write(dirs.root().join("def_test"), data).expect("Unable to write file");
        let actual = nu!(
            cwd: dirs.root(),
            "source def_test; help e | to json -r"
        );

        assert!(actual.out.contains("My echo\\n\\n"));
    });
}
