use nu_test_support::{nu, pipeline};

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
