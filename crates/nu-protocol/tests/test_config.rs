use nu_test_support::{nu, nu_repl_code};

#[test]
fn filesize_metric_true() {
    let code = &[
        r#"$env.config = { filesize: { metric: true, format:"mb" } }"#,
        r#"20mib | into string"#,
    ];
    let actual = nu!(cwd: ".", nu_repl_code( code ));
    assert_eq!(actual.out, "21.0 MB");
}

#[test]
fn filesize_metric_false() {
    let code = &[
        r#"$env.config = { filesize: { metric: false, format:"mib" } }"#,
        r#"20mib | into string"#,
    ];
    let actual = nu!(cwd: ".", nu_repl_code( code ));
    assert_eq!(actual.out, "20.0 MiB");
}

#[test]
fn filesize_metric_overrides_format() {
    let code = &[
        r#"$env.config = { filesize: { metric: false, format:"mb" } }"#,
        r#"20mib | into string"#,
    ];
    let actual = nu!(cwd: ".", nu_repl_code( code ));
    assert_eq!(actual.out, "20.0 MiB");
}

#[test]
fn filesize_format_auto_metric_true() {
    let code = &[
        r#"$env.config = { filesize: { metric: true, format:"auto" } }"#,
        r#"[2mb 2gb 2tb] | into string | to nuon"#,
    ];
    let actual = nu!(cwd: ".", nu_repl_code( code ));
    assert_eq!(actual.out, r#"["2.0 MB", "2.0 GB", "2.0 TB"]"#);
}

#[test]
fn filesize_format_auto_metric_false() {
    let code = &[
        r#"$env.config = { filesize: { metric: false, format:"auto" } }"#,
        r#"[2mb 2gb 2tb] | into string | to nuon"#,
    ];
    let actual = nu!(cwd: ".", nu_repl_code( code ));
    assert_eq!(actual.out, r#"["1.9 MiB", "1.9 GiB", "1.8 TiB"]"#);
}
