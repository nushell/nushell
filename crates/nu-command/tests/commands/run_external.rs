use nu_test_support::{nu, pipeline};
use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;

#[test]
fn better_empty_redirection() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            ls | each { |it| nu --testbin cococo $it.name }
        "#
    ));

    eprintln!("out: {}", actual.out);

    assert!(!actual.out.contains('2'));
}

#[test]
fn explicit_glob() {
    Playground::setup("external with explicit glob", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^ls | glob '*.txt' | length
            "#
        ));

        assert_eq!(actual.out, "2");
    })
}

