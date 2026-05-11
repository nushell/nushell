use nu_protocol::test_record;
use nu_test_support::{nu, nu_repl_code, prelude::*};
use rstest::rstest;

#[rstest]
#[case::mb("MB")]
#[case::mib("MiB")]
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
    let code = nu_repl_code(&[
        "$env.config.use_ansi_coloring = true",
        r#"def force_error [x] {
        error make {
            msg: "oh no!"
            label: {
                text: "here's the error"
                span: (metadata $x).span
            }
        }
    }"#,
        r#"force_error "My error""#,
    ]);

    let actual = nu!(format!("try {{ {code} }}"));

    assert_eq!(
        actual.err,
        "Error: \u{1b}[31mnu::shell::error\u{1b}[0m\n\n  \u{1b}[31m×\u{1b}[0m oh no!\n   ╭─[\u{1b}[36;1;4mline2:1:13\u{1b}[0m]\n \u{1b}[2m1\u{1b}[0m │ force_error \"My error\"\n   · \u{1b}[35;1m            ─────┬────\u{1b}[0m\n   ·                  \u{1b}[35;1m╰── \u{1b}[35;1mhere's the error\u{1b}[0m\u{1b}[0m\n   ╰────\n\n"
    );

    Ok(())
}

#[test]
fn narratable_errors() -> Result {
    let code = nu_repl_code(&[
        r#"$env.config = { error_style: "plain" }"#,
        r#"def force_error [x] {
        error make {
            msg: "oh no!"
            label: {
                text: "here's the error"
                span: (metadata $x).span
            }
        }
    }"#,
        r#"force_error "my error""#,
    ]);

    let actual = nu!(format!("try {{ {code} }}"));

    assert_eq!(
        actual.err,
        r#"Error: oh no!
    Diagnostic severity: error
Begin snippet for line2 starting at line 1, column 1

snippet line 1: force_error "my error"
    label at line 1, columns 13 to 22: here's the error
diagnostic code: nu::shell::error


"#,
    );

    Ok(())
}

#[test]
fn plugins() -> Result {
    let mut tester = test();
    let () = tester.run("$env.config = { plugins: { nu_plugin_config: { key: value } } }")?;
    tester
        .run("$env.config.plugins")
        .expect_value_eq(test_record! {
            "nu_plugin_config" => test_record! {
                "key" => "value"
            }
        })
}
