use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, nu_repl_code, pipeline};

#[test]
fn add_overlay() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use spam"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn add_overlay_as_new_name() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use spam as spam_new"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn add_overlay_twice() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use spam"#,
        r#"overlay use spam"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn add_prefixed_overlay() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use --prefix spam"#,
        r#"spam foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn add_prefixed_overlay_twice() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use --prefix spam"#,
        r#"overlay use --prefix spam"#,
        r#"spam foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn add_prefixed_overlay_mismatch_1() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use --prefix spam"#,
        r#"overlay use spam"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(actual.err.contains("exists with a prefix"));
    // Why doesn't the REPL test work with the previous expected output
    assert!(actual_repl.err.contains("overlay_prefix_mismatch"));
}

#[test]
fn add_prefixed_overlay_mismatch_2() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use spam"#,
        r#"overlay use --prefix spam"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(actual.err.contains("exists without a prefix"));
    // Why doesn't the REPL test work with the previous expected output
    assert!(actual_repl.err.contains("overlay_prefix_mismatch"));
}

#[test]
fn prefixed_overlay_keeps_custom_decl() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use --prefix spam"#,
        r#"def bar [] { "bar" }"#,
        r#"overlay hide --keep-custom spam"#,
        r#"bar"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "bar");
    assert_eq!(actual_repl.out, "bar");
}

#[test]
fn add_overlay_env() {
    let inp = &[
        r#"module spam { export-env { let-env FOO = "foo" } }"#,
        r#"overlay use spam"#,
        r#"$env.FOO"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn add_prefixed_overlay_env_no_prefix() {
    let inp = &[
        r#"module spam { export-env { let-env FOO = "foo" } }"#,
        r#"overlay use --prefix spam"#,
        r#"$env.FOO"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn add_overlay_from_file_decl() {
    let inp = &[r#"overlay use samples/spam.nu"#, r#"foo"#];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

// This one tests that the `nu_repl()` loop works correctly
#[test]
fn add_overlay_from_file_decl_cd() {
    let inp = &[r#"cd samples"#, r#"overlay use spam.nu"#, r#"foo"#];

    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn add_overlay_from_file_alias() {
    let inp = &[r#"overlay use samples/spam.nu"#, r#"bar"#];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "bar");
    assert_eq!(actual_repl.out, "bar");
}

#[test]
fn add_overlay_from_file_env() {
    let inp = &[r#"overlay use samples/spam.nu"#, r#"$env.BAZ"#];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "baz");
    assert_eq!(actual_repl.out, "baz");
}

#[test]
fn add_overlay_scoped() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"do { overlay use spam }"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

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
        r#"overlay use spam"#,
        r#"module spam { export def foo [] { "bar" } }"#,
        r#"overlay use spam"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "bar");
    assert_eq!(actual_repl.out, "bar");
}

#[test]
fn update_overlay_from_module_env() {
    let inp = &[
        r#"module spam { export-env { let-env FOO = "foo" } }"#,
        r#"overlay use spam"#,
        r#"module spam { export-env { let-env FOO = "bar" } }"#,
        r#"overlay use spam"#,
        r#"$env.FOO"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "bar");
    assert_eq!(actual_repl.out, "bar");
}

#[test]
fn overlay_use_do_not_eval_twice() {
    let inp = &[
        r#"module spam { export-env { let-env FOO = "foo" } }"#,
        r#"overlay use spam"#,
        r#"let-env FOO = "bar""#,
        r#"overlay hide spam"#,
        r#"overlay use spam"#,
        r#"$env.FOO"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "bar");
    assert_eq!(actual_repl.out, "bar");
}

#[test]
fn remove_overlay() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use spam"#,
        r#"overlay hide spam"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

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
        r#"overlay use spam"#,
        r#"overlay hide"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

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
        r#"overlay use spam"#,
        r#"do { overlay hide spam }"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn remove_overlay_env() {
    let inp = &[
        r#"module spam { export-env { let-env FOO = "foo" } }"#,
        r#"overlay use spam"#,
        r#"overlay hide spam"#,
        r#"$env.FOO"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(actual.err.contains("cannot find column"));
    assert!(actual_repl.err.contains("cannot find column"));
}

#[test]
fn remove_overlay_scoped_env() {
    let inp = &[
        r#"module spam { export-env { let-env FOO = "foo" } }"#,
        r#"overlay use spam"#,
        r#"do { overlay hide spam }"#,
        r#"$env.FOO"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn list_default_overlay() {
    let inp = &[r#"overlay list | last"#];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "zero");
    assert_eq!(actual_repl.out, "zero");
}

#[test]
fn list_last_overlay() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use spam"#,
        r#"overlay list | last"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "spam");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn list_overlay_scoped() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use spam"#,
        r#"do { overlay list | last }"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "spam");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn remove_overlay_discard_decl() {
    let inp = &[
        r#"overlay use samples/spam.nu"#,
        r#"def bagr [] { "bagr" }"#,
        r#"overlay hide spam"#,
        r#"bagr"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(!actual.err.is_empty());
    #[cfg(windows)]
    assert!(actual_repl.out != "bagr");
    #[cfg(not(windows))]
    assert!(!actual_repl.err.is_empty());
}

#[test]
fn remove_overlay_discard_alias() {
    let inp = &[
        r#"overlay use samples/spam.nu"#,
        r#"alias bagr = "bagr""#,
        r#"overlay hide spam"#,
        r#"bagr"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(!actual.err.is_empty());
    #[cfg(windows)]
    assert!(actual_repl.out != "bagr");
    #[cfg(not(windows))]
    assert!(!actual_repl.err.is_empty());
}

#[test]
fn remove_overlay_discard_env() {
    let inp = &[
        r#"overlay use samples/spam.nu"#,
        r#"let-env BAGR = `bagr`"#,
        r#"overlay hide spam"#,
        r#"$env.BAGR"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(actual.err.contains("cannot find column"));
    assert!(actual_repl.err.contains("cannot find column"));
}

#[test]
fn remove_overlay_keep_decl() {
    let inp = &[
        r#"overlay use samples/spam.nu"#,
        r#"def bagr [] { "bagr" }"#,
        r#"overlay hide --keep-custom spam"#,
        r#"bagr"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(actual.out.contains("bagr"));
    assert!(actual_repl.out.contains("bagr"));
}

#[test]
fn remove_overlay_keep_alias() {
    let inp = &[
        r#"overlay use samples/spam.nu"#,
        r#"alias bagr = `bagr`"#,
        r#"overlay hide --keep-custom spam"#,
        r#"bagr"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(actual.out.contains("bagr"));
    assert!(actual_repl.out.contains("bagr"));
}

#[test]
fn remove_overlay_dont_keep_env() {
    let inp = &[
        r#"overlay use samples/spam.nu"#,
        r#"let-env BAGR = `bagr`"#,
        r#"overlay hide --keep-custom spam"#,
        r#"$env.BAGR"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(actual.err.contains("cannot find column"));
    assert!(actual_repl.err.contains("cannot find column"));
}

#[test]
fn remove_overlay_dont_keep_overwritten_decl() {
    let inp = &[
        r#"overlay use samples/spam.nu"#,
        r#"def foo [] { 'bar' }"#,
        r#"overlay hide --keep-custom spam"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(!actual.err.is_empty());
    #[cfg(windows)]
    assert!(actual_repl.out != "bagr");
    #[cfg(not(windows))]
    assert!(!actual_repl.err.is_empty());
}

#[test]
fn remove_overlay_dont_keep_overwritten_alias() {
    let inp = &[
        r#"overlay use samples/spam.nu"#,
        r#"alias bar = `baz`"#,
        r#"overlay hide --keep-custom spam"#,
        r#"bar"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(!actual.err.is_empty());
    #[cfg(windows)]
    assert!(actual_repl.out != "bagr");
    #[cfg(not(windows))]
    assert!(!actual_repl.err.is_empty());
}

#[test]
fn remove_overlay_dont_keep_overwritten_env() {
    let inp = &[
        r#"overlay use samples/spam.nu"#,
        r#"let-env BAZ = `bagr`"#,
        r#"overlay hide --keep-custom spam"#,
        r#"$env.BAZ"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(actual.err.contains("cannot find column"));
    assert!(actual_repl.err.contains("cannot find column"));
}

#[test]
fn remove_overlay_keep_decl_in_latest_overlay() {
    let inp = &[
        r#"overlay use samples/spam.nu"#,
        r#"def bagr [] { 'bagr' }"#,
        r#"module eggs { }"#,
        r#"overlay use eggs"#,
        r#"overlay hide --keep-custom spam"#,
        r#"bagr"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(actual.out.contains("bagr"));
    assert!(actual_repl.out.contains("bagr"));
}

#[test]
fn remove_overlay_keep_alias_in_latest_overlay() {
    let inp = &[
        r#"overlay use samples/spam.nu"#,
        r#"alias bagr = `bagr`"#,
        r#"module eggs { }"#,
        r#"overlay use eggs"#,
        r#"overlay hide --keep-custom spam"#,
        r#"bagr"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(actual.out.contains("bagr"));
    assert!(actual_repl.out.contains("bagr"));
}

#[test]
fn remove_overlay_dont_keep_env_in_latest_overlay() {
    let inp = &[
        r#"overlay use samples/spam.nu"#,
        r#"let-env BAGR = `bagr`"#,
        r#"module eggs { }"#,
        r#"overlay use eggs"#,
        r#"overlay hide --keep-custom spam"#,
        r#"$env.BAGR"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(actual.err.contains("cannot find column"));
    assert!(actual_repl.err.contains("cannot find column"));
}

#[test]
fn preserve_overrides() {
    let inp = &[
        r#"overlay use samples/spam.nu"#,
        r#"def foo [] { "new-foo" }"#,
        r#"overlay hide spam"#,
        r#"overlay use spam"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "new-foo");
    assert_eq!(actual_repl.out, "new-foo");
}

#[test]
fn reset_overrides() {
    let inp = &[
        r#"overlay use samples/spam.nu"#,
        r#"def foo [] { "new-foo" }"#,
        r#"overlay hide spam"#,
        r#"overlay use samples/spam.nu"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn overlay_new() {
    let inp = &[r#"overlay new spam"#, r#"overlay list | last"#];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "spam");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn overlay_keep_pwd() {
    let inp = &[
        r#"overlay new spam"#,
        r#"cd samples"#,
        r#"overlay hide --keep-env [ PWD ] spam"#,
        r#"$env.PWD | path basename"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "samples");
    assert_eq!(actual_repl.out, "samples");
}

#[test]
fn overlay_wrong_rename_type() {
    let inp = &[r#"module spam {}"#, r#"overlay use spam as { echo foo }"#];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));

    assert!(actual.err.contains("parse_mismatch"));
}

#[test]
fn overlay_add_renamed() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use spam as eggs --prefix"#,
        r#"eggs foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn overlay_add_renamed_from_file() {
    let inp = &[
        r#"overlay use samples/spam.nu as eggs --prefix"#,
        r#"eggs foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn overlay_cant_rename_existing_overlay() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use spam"#,
        r#"overlay hide spam"#,
        r#"overlay use spam as eggs"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(actual.err.contains("cant_add_overlay_help"));
    assert!(actual_repl.err.contains("cant_add_overlay_help"));
}

#[test]
fn overlay_can_add_renamed_overlay() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use spam as eggs --prefix"#,
        r#"overlay use spam --prefix"#,
        r#"(spam foo) + (eggs foo)"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foofoo");
    assert_eq!(actual_repl.out, "foofoo");
}

#[test]
fn overlay_remove_renamed_overlay() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use spam as eggs"#,
        r#"overlay hide eggs"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(actual.err.contains("did you mean 'for'?"));
    assert!(actual_repl.err.contains("did you mean 'for'?"));
}

#[test]
fn overlay_remove_and_add_renamed_overlay() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use spam as eggs"#,
        r#"overlay hide eggs"#,
        r#"overlay use eggs"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn overlay_use_export_env() {
    let inp = &[
        r#"module spam { export-env { let-env FOO = 'foo' } }"#,
        r#"overlay use spam"#,
        r#"$env.FOO"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn overlay_use_export_env_hide() {
    let inp = &[
        r#"let-env FOO = 'foo'"#,
        r#"module spam { export-env { hide-env FOO } }"#,
        r#"overlay use spam"#,
        r#"$env.FOO"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(actual.err.contains("cannot find column"));
    assert!(actual_repl.err.contains("cannot find column"));
}

#[test]
fn overlay_use_do_cd() {
    Playground::setup("overlay_use_do_cd", |dirs, sandbox| {
        sandbox
            .mkdir("test1/test2")
            .with_files(vec![FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                r#"
                    export-env { cd test1/test2 }
                "#,
            )]);

        let inp = &[
            r#"overlay use test1/test2/spam.nu"#,
            r#"$env.PWD | path basename"#,
        ];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert_eq!(actual.out, "test2");
    })
}

#[test]
fn overlay_use_do_cd_file_relative() {
    Playground::setup("overlay_use_do_cd_file_relative", |dirs, sandbox| {
        sandbox
            .mkdir("test1/test2")
            .with_files(vec![FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                r#"
                    export-env { cd ($env.FILE_PWD | path join '..') }
                "#,
            )]);

        let inp = &[
            r#"overlay use test1/test2/spam.nu"#,
            r#"$env.PWD | path basename"#,
        ];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert_eq!(actual.out, "test1");
    })
}

#[test]
fn overlay_use_dont_cd_overlay() {
    Playground::setup("overlay_use_dont_cd_overlay", |dirs, sandbox| {
        sandbox
            .mkdir("test1/test2")
            .with_files(vec![FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                r#"
                    export-env {
                        overlay new spam
                        cd test1/test2
                        overlay hide spam
                    }
                "#,
            )]);

        let inp = &[
            r#"source-env test1/test2/spam.nu"#,
            r#"$env.PWD | path basename"#,
        ];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert_eq!(actual.out, "overlay_use_dont_cd_overlay");
    })
}

#[test]
fn overlay_use_find_scoped_module() {
    Playground::setup("overlay_use_find_module_scoped", |dirs, _| {
        let inp = r#"
                do {
                    module spam { }

                    overlay use spam
                    overlay list | last
                }
            "#;

        let actual = nu!(cwd: dirs.test(), inp);

        assert_eq!(actual.out, "spam");
    })
}

#[test]
fn overlay_preserve_hidden_env_1() {
    let inp = &[
        r#"overlay new spam"#,
        r#"let-env FOO = 'foo'"#,
        r#"overlay new eggs"#,
        r#"let-env FOO = 'bar'"#,
        r#"hide-env FOO"#,
        r#"overlay use eggs"#,
        r#"$env.FOO"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn overlay_preserve_hidden_env_2() {
    let inp = &[
        r#"overlay new spam"#,
        r#"let-env FOO = 'foo'"#,
        r#"overlay hide spam"#,
        r#"overlay new eggs"#,
        r#"let-env FOO = 'bar'"#,
        r#"hide-env FOO"#,
        r#"overlay hide eggs"#,
        r#"overlay use spam"#,
        r#"overlay use eggs"#,
        r#"$env.FOO"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn overlay_reset_hidden_env() {
    let inp = &[
        r#"overlay new spam"#,
        r#"let-env FOO = 'foo'"#,
        r#"overlay new eggs"#,
        r#"let-env FOO = 'bar'"#,
        r#"hide-env FOO"#,
        r#"module eggs { export-env { let-env FOO = 'bar' } }"#,
        r#"overlay use eggs"#,
        r#"$env.FOO"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "bar");
    assert_eq!(actual_repl.out, "bar");
}

#[ignore = "TODO: For this to work, we'd need to make predecls respect overlays"]
#[test]
fn overlay_preserve_hidden_decl() {
    let inp = &[
        r#"overlay new spam"#,
        r#"def foo [] { 'foo' }"#,
        r#"overlay new eggs"#,
        r#"def foo [] { 'bar' }"#,
        r#"hide foo"#,
        r#"overlay use eggs"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn overlay_preserve_hidden_alias() {
    let inp = &[
        r#"overlay new spam"#,
        r#"alias foo = 'foo'"#,
        r#"overlay new eggs"#,
        r#"alias foo = 'bar'"#,
        r#"hide foo"#,
        r#"overlay use eggs"#,
        r#"foo"#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn overlay_trim_single_quote() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use 'spam'"#,
        r#"overlay list | last "#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));

    assert_eq!(actual.out, "spam");
}

#[test]
fn overlay_trim_single_quote_hide() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use 'spam'"#,
        r#"overlay hide spam "#,
        r#"foo"#,
    ];
    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(!actual.err.is_empty());
    #[cfg(windows)]
    assert!(actual_repl.out != "foo");
    #[cfg(not(windows))]
    assert!(!actual_repl.err.is_empty());
}

#[test]
fn overlay_trim_double_quote() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use "spam" "#,
        r#"overlay list | last "#,
    ];

    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));

    assert_eq!(actual.out, "spam");
}

#[test]
fn overlay_trim_double_quote_hide() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use "spam" "#,
        r#"overlay hide spam "#,
        r#"foo"#,
    ];
    let actual = nu!(cwd: "tests/overlays", pipeline(&inp.join("; ")));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(!actual.err.is_empty());
    #[cfg(windows)]
    assert!(actual_repl.out != "foo");
    #[cfg(not(windows))]
    assert!(!actual_repl.err.is_empty());
}
