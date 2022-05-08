use super::nu_repl::nu_repl;
use nu_test_support::{nu, pipeline};

#[test]
fn add_overlay() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay add spam"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu_repl("tests/overlays", inp);

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn add_overlay_env() {
    let inp = &[
        r#"module spam { export env FOO { "foo" } }"#,
        r#"overlay add spam"#,
        r#"$env.FOO"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu_repl("tests/overlays", inp);

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn add_overlay_from_file_decl() {
    let inp = &[r#"overlay add samples/spam.nu"#, r#"foo"#];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu_repl("tests/overlays", inp);

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

// This one tests that the `nu_repl()` loop works correctly
#[test]
fn add_overlay_from_file_decl_cd() {
    let inp = &[r#"cd samples"#, r#"overlay add spam.nu"#, r#"foo"#];

    let actual_repl = nu_repl("tests/overlays", inp);

    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn add_overlay_from_file_alias() {
    let inp = &[r#"overlay add samples/spam.nu"#, r#"bar"#];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu_repl("tests/overlays", inp);

    assert_eq!(actual.out, "bar");
    assert_eq!(actual_repl.out, "bar");
}

#[test]
fn add_overlay_from_file_env() {
    let inp = &[r#"overlay add samples/spam.nu"#, r#"$env.BAZ"#];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu_repl("tests/overlays", inp);

    assert_eq!(actual.out, "baz");
    assert_eq!(actual_repl.out, "baz");
}

#[test]
fn add_overlay_scoped() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"do { overlay add spam }"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu_repl("tests/overlays", inp);

    assert!(!actual.err.is_empty());
    #[cfg(windows)]
    assert!(actual_repl.out != "foo");
    #[cfg(not(windows))]
    assert!(!actual_repl.err.is_empty());
}

#[test]
fn update_overlay_from_module() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay add spam"#,
        r#"module spam { export def foo [] { "bar" } }"#,
        r#"overlay add spam"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu_repl("tests/overlays", inp);

    assert_eq!(actual.out, "bar");
    assert_eq!(actual_repl.out, "bar");
}

#[test]
fn update_overlay_from_module_env() {
    let inp = &[
        r#"module spam { export env FOO { "foo" } }"#,
        r#"overlay add spam"#,
        r#"module spam { export env FOO { "bar" } }"#,
        r#"overlay add spam"#,
        r#"$env.FOO"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu_repl("tests/overlays", inp);

    assert_eq!(actual.out, "bar");
    assert_eq!(actual_repl.out, "bar");
}

#[test]
fn remove_overlay() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay add spam"#,
        r#"overlay remove spam"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu_repl("tests/overlays", inp);

    assert!(!actual.err.is_empty());
    #[cfg(windows)]
    assert!(actual_repl.out != "foo");
    #[cfg(not(windows))]
    assert!(!actual_repl.err.is_empty());
}

#[test]
fn remove_last_overlay() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay add spam"#,
        r#"overlay remove"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu_repl("tests/overlays", inp);

    assert!(!actual.err.is_empty());
    #[cfg(windows)]
    assert!(actual_repl.out != "foo");
    #[cfg(not(windows))]
    assert!(!actual_repl.err.is_empty());
}

#[test]
fn remove_overlay_scoped() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay add spam"#,
        r#"do { overlay remove spam }"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu_repl("tests/overlays", inp);

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn remove_overlay_env() {
    let inp = &[
        r#"module spam { export env FOO { "foo" } }"#,
        r#"overlay add spam"#,
        r#"overlay remove spam"#,
        r#"$env.FOO"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu_repl("tests/overlays", inp);

    assert!(actual.err.contains("did you mean"));
    assert!(actual_repl.err.contains("DidYouMean"));
}

#[test]
fn remove_overlay_scoped_env() {
    let inp = &[
        r#"module spam { export env FOO { "foo" } }"#,
        r#"overlay add spam"#,
        r#"do { overlay remove spam }"#,
        r#"$env.FOO"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu_repl("tests/overlays", inp);

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn list_default_overlay() {
    let inp = &[r#"overlay list | last"#];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu_repl("tests/overlays", inp);

    assert_eq!(actual.out, "zero");
    assert_eq!(actual_repl.out, "zero");
}

#[test]
fn list_last_overlay() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay add spam"#,
        r#"overlay list | last"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu_repl("tests/overlays", inp);

    assert_eq!(actual.out, "spam");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn list_overlay_scoped() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay add spam"#,
        r#"do { overlay list | last }"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu_repl("tests/overlays", inp);

    assert_eq!(actual.out, "spam");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn remove_overlay_discard_decl() {
    let inp = &[
        r#"overlay add samples/spam.nu"#,
        r#"def bagr [] { "bagr" }"#,
        r#"overlay remove spam"#,
        r#"bagr"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu_repl("tests/overlays", inp);

    assert!(!actual.err.is_empty());
    #[cfg(windows)]
    assert!(actual_repl.out != "bagr");
    #[cfg(not(windows))]
    assert!(!actual_repl.err.is_empty());
}

#[test]
fn remove_overlay_discard_alias() {
    let inp = &[
        r#"overlay add samples/spam.nu"#,
        r#"alias bagr = "bagr""#,
        r#"overlay remove spam"#,
        r#"bagr"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu_repl("tests/overlays", inp);

    assert!(!actual.err.is_empty());
    #[cfg(windows)]
    assert!(actual_repl.out != "bagr");
    #[cfg(not(windows))]
    assert!(!actual_repl.err.is_empty());
}

#[test]
fn remove_overlay_discard_env() {
    let inp = &[
        r#"overlay add samples/spam.nu"#,
        r#"let-env BAGR = "bagr""#,
        r#"overlay remove spam"#,
        r#"$env.BAGR"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu_repl("tests/overlays", inp);

    assert!(actual.err.contains("did you mean"));
    assert!(actual_repl.err.contains("DidYouMean"));
}

#[test]
fn preserve_overrides() {
    let inp = &[
        r#"overlay add samples/spam.nu"#,
        r#"def foo [] { "new-foo" }"#,
        r#"overlay remove spam"#,
        r#"overlay add spam"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu_repl("tests/overlays", inp);

    assert_eq!(actual.out, "new-foo");
    assert_eq!(actual_repl.out, "new-foo");
}

#[test]
fn reset_overrides() {
    let inp = &[
        r#"overlay add samples/spam.nu"#,
        r#"def foo [] { "new-foo" }"#,
        r#"overlay remove spam"#,
        r#"overlay add samples/spam.nu"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu_repl("tests/overlays", inp);

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}
