use nu_test_support::prelude::*;
use rstest::rstest;

#[rstest]
#[case::major_minor("--major --minor")]
#[case::minor_major("--minor --major")] // regardless of order of arguments
#[nu_test_support::test]
#[deps(NU_PLUGIN_INC)]
fn chooses_highest_increment_if_given_more_than_one(#[case] args: &str) -> Result {
    let code = format! {"
        open cargo_sample.toml
        | inc package.version {args}
        | get package.version
    "};

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("1.0.0")
}

#[test]
#[deps(NU_PLUGIN_INC)]
fn by_one_with_field_passed(playground: Playground) -> Result {
    playground.file(
        "sample.toml",
        r#"
            [package]
            edition = "2018" 
        "#,
    )?;

    test()
        .cwd(playground.path())
        .run("open sample.toml | inc package.edition | get package.edition")
        .expect_value_eq("2019")
}

#[test]
#[deps(NU_PLUGIN_INC)]
fn by_one_with_no_field_passed(playground: Playground) -> Result {
    playground.file(
        "sample.toml",
        r#"
            [package]
            contributors = "2" 
        "#,
    )?;

    test()
        .cwd(playground.path())
        .run("open sample.toml | get package.contributors | inc")
        .expect_value_eq("3")
}

#[rstest]
#[case::major("-M", "1.0.0")]
#[case::minor("--minor", "0.2.0")]
#[case::patch("--patch", "0.1.4")]
#[nu_test_support::test]
#[deps(NU_PLUGIN_INC)]
fn semantic_version_inc(
    #[ignore] playground: Playground,
    #[case] args: &str,
    #[case] expected: &str,
) -> Result {
    playground.file(
        "sample.toml",
        r#"
            [package]
            version = "0.1.3"  
        "#,
    )?;

    let code = format! {"
        open sample.toml
        | inc package.version {args}
        | get package.version
    "};

    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq(expected)
}

#[test]
#[deps(NU_PLUGIN_INC)]
fn semantic_version_without_passing_field(playground: Playground) -> Result {
    playground.file(
        "sample.toml",
        r#"
            [package]
            version = "0.1.3"
        "#,
    )?;

    test()
        .cwd(playground.path())
        .run("open sample.toml | get package.version | inc --patch")
        .expect_value_eq("0.1.4")
}

#[test]
#[deps(NU_PLUGIN_INC)]
fn explicit_flag() -> Result {
    test()
        .run("'0.1.2' | inc --major=false --minor=true --patch=false")
        .expect_value_eq("0.2.0")
}
