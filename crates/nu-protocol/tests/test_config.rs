use nu_test_support::prelude::*;
use rstest::rstest;

#[rstest]
#[case::mb("MB")]
#[case::mib("MiB")]
#[nu_test_support::test]
#[env(NU_TEST_LOCALE_OVERRIDE = "en_US.UTF-8")]
fn filesize(#[case] unit: &str) -> Result {
    let mut tester = test();
    let () = tester.run_with_data("$env.config.filesize.unit = $in", unit)?;
    tester
        .run(format!("20{unit} | into string"))
        .expect_value_eq(format!("20.0 {unit}"))
}

#[rstest]
#[case::metric("metric", "[2MB, 2GB, 2TB]", &["2.0 MB", "2.0 GB", "2.0 TB"])]
#[case::binary("binary", "[2MiB, 2GiB, 2TiB]", &["2.0 MiB", "2.0 GiB", "2.0 TiB"])]
#[nu_test_support::test]
#[env(NU_TEST_LOCALE_OVERRIDE = "en_US.UTF-8")]
fn filesize_format(#[case] unit: &str, #[case] input: &str, #[case] expected: &[&str]) -> Result {
    let mut tester = test();
    let () = tester.run_with_data("$env.config.filesize.unit = $in", unit)?;
    let val: Value = tester.run(input)?;
    tester
        .run_with_data("into string", val)
        .expect_value_eq(expected.to_vec())
}

#[test]
fn fancy_default_errors() -> Result {
    let mut tester = test();
    let () = tester.run("$env.config.use_ansi_coloring = true")?;
    let () = tester.run(
        r#"def force_error [x] {
            error make {
                msg: "oh no!"
                label: {
                    text: "here's the error"
                    span: (metadata $x).span
                }
            }
        }"#,
    )?;

    let err = tester
        .run(r#"force_error "My error""#)
        .expect_labeled_error()?;

    assert_eq!(err.msg, "oh no!");
    assert_eq!(err.labels.len(), 1);
    assert_eq!(err.labels[0].text, "here's the error");

    Ok(())
}

#[test]
fn narratable_errors() -> Result {
    let mut tester = test();
    let () = tester.run(r#"$env.config = { error_style: "plain" }"#)?;
    let () = tester.run(
        r#"def force_error [x] {
            error make {
                msg: "oh no!"
                label: {
                    text: "here's the error"
                    span: (metadata $x).span
                }
            }
        }"#,
    )?;

    let err = tester
        .run(r#"force_error "my error""#)
        .expect_labeled_error()?;

    assert_eq!(err.msg, "oh no!");
    assert_eq!(err.labels.len(), 1);
    assert_eq!(err.labels[0].text, "here's the error");

    Ok(())
}

#[test]
fn abbreviations() -> Result {
    let mut tester = test();
    let () = tester.run(r#"$env.config = { abbreviations: { g: "git --no-pager" } }"#)?;
    tester
        .run("$env.config.abbreviations")
        .expect_value_eq(test_value!({
            "g": "git --no-pager"
        }))
}

#[test]
fn plugins() -> Result {
    let mut tester = test();
    let () = tester.run("$env.config = { plugins: { nu_plugin_config: { key: value } } }")?;
    tester
        .run("$env.config.plugins")
        .expect_value_eq(test_value!({
            "nu_plugin_config": {
                "key": "value"
            }
        }))
}
