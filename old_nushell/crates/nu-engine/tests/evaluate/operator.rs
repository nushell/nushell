use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn filter_ls_by_in_array() {
    Playground::setup("filter_ls_by_1", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("jean-luc.cap"),
            EmptyFile("riker.cmdr"),
            EmptyFile("data.ltcmdr"),
            EmptyFile("troi.ltcmdr"),
            EmptyFile("worf.lt"),
            EmptyFile("geordi.lt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls | where name in ['data.ltcmdr', 'riker.cmdr'] | get name | to json
            "#
        ));

        assert_eq!(actual.out, "[\"data.ltcmdr\",\"riker.cmdr\"]");
    })
}

#[test]
fn filter_json_with_in_array() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo '[{"name": "foo", "size": 3}, {"name": "foo", "size": 2}, {"name": "bar", "size": 4}]' | from json | where size in [2] | get name
        "#
    ));

    assert_eq!(actual.out, "foo");
}
