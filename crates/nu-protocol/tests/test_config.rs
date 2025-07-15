use nu_test_support::{nu, nu_repl_code};

#[test]
fn filesize_mb() {
    let code = &[
        r#"$env.config = { filesize: { unit: MB } }"#,
        r#"20MB | into string"#,
    ];
    let actual = nu!(nu_repl_code(code));
    assert_eq!(actual.out, "20.0 MB");
}

#[test]
fn filesize_mib() {
    let code = &[
        r#"$env.config = { filesize: { unit: MiB } }"#,
        r#"20MiB | into string"#,
    ];
    let actual = nu!(nu_repl_code(code));
    assert_eq!(actual.out, "20.0 MiB");
}

#[test]
fn filesize_format_decimal() {
    let code = &[
        r#"$env.config = { filesize: { unit: metric } }"#,
        r#"[2MB 2GB 2TB] | into string | to nuon"#,
    ];
    let actual = nu!(nu_repl_code(code));
    assert_eq!(actual.out, r#"["2.0 MB", "2.0 GB", "2.0 TB"]"#);
}

#[test]
fn filesize_format_binary() {
    let code = &[
        r#"$env.config = { filesize: { unit: binary } }"#,
        r#"[2MiB 2GiB 2TiB] | into string | to nuon"#,
    ];
    let actual = nu!(nu_repl_code(code));
    assert_eq!(actual.out, r#"["2.0 MiB", "2.0 GiB", "2.0 TiB"]"#);
}

#[test]
fn fancy_default_errors() {
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
}

#[test]
fn narratable_errors() {
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
}

#[test]
fn plugins() {
    let code = &[
        r#"$env.config = { plugins: { nu_plugin_config: { key: value } } }"#,
        r#"$env.config.plugins"#,
    ];
    let actual = nu!(nu_repl_code(code));
    assert_eq!(actual.out, r#"{nu_plugin_config: {key: value}}"#);
}
