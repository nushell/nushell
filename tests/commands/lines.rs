use nu_test_support::{nu, pipeline};

#[test]
fn lines() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml --raw
            | lines
            | skip-while $it != "[dependencies]"
            | skip 1
            | first 1
            | split-column "="
            | get Column1
            | trim
            | echo $it
        "#
    ));

    assert_eq!(actual, "rustyline");
}
