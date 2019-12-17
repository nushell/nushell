use nu_test_support::{nu, pipeline};

#[test]
fn by_column() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml --raw
            | lines
            | skip 1
            | first 4
            | split-column "="
            | sort-by Column1
            | skip 1
            | first 1
            | get Column1
            | trim
            | echo $it
        "#
    ));

    assert_eq!(actual, "description");
}
