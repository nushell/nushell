use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn gets_first_row_when_no_amount_given() {
    Playground::setup("first_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("caballeros.txt"), EmptyFile("arepas.clu")]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | first
                | length
            "#
        ));

        assert_eq!(actual.out, "1");
    })
}

#[test]
// covers a situation where `first` used to behave strangely on list<binary> input
fn works_with_binary_list() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        ([0x[01 11]] | first) == 0x[01 11]
            "#
    ));

    assert_eq!(actual.out, "true");
}
