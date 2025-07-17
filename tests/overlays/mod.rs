use nu_test_support::fs::Stub::{FileWithContent, FileWithContentToBeTrimmed};
use nu_test_support::playground::Playground;
use nu_test_support::{nu, nu_repl_code};
use pretty_assertions::assert_eq;

#[test]
fn add_overlay() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam",
        "foo",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn add_overlay_as_new_name() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam as spam_new",
        "foo",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn add_overlay_twice() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam",
        "overlay use spam",
        "foo",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn add_prefixed_overlay() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use --prefix spam",
        "spam foo",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn add_prefixed_overlay_twice() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use --prefix spam",
        "overlay use --prefix spam",
        "spam foo",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn add_prefixed_overlay_mismatch_1() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use --prefix spam",
        "overlay use spam",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(actual.err.contains("exists with a prefix"));
    // Why doesn't the REPL test work with the previous expected output
    assert!(actual_repl.err.contains("overlay_prefix_mismatch"));
}

#[test]
fn add_prefixed_overlay_mismatch_2() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam",
        "overlay use --prefix spam",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(actual.err.contains("exists without a prefix"));
    // Why doesn't the REPL test work with the previous expected output
    assert!(actual_repl.err.contains("overlay_prefix_mismatch"));
}

#[test]
fn prefixed_overlay_keeps_custom_decl() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use --prefix spam",
        r#"def bar [] { "bar" }"#,
        "overlay hide --keep-custom spam",
        "bar",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "bar");
    assert_eq!(actual_repl.out, "bar");
}

#[test]
fn add_overlay_env() {
    let inp = &[
        r#"module spam { export-env { $env.FOO = "foo" } }"#,
        "overlay use spam",
        "$env.FOO",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn add_prefixed_overlay_env_no_prefix() {
    let inp = &[
        r#"module spam { export-env { $env.FOO = "foo" } }"#,
        "overlay use --prefix spam",
        "$env.FOO",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn add_overlay_from_file_decl() {
    let inp = &["overlay use samples/spam.nu", "foo"];

    let actual = nu!(cwd: "tests/overlays", &inp.join("; "));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn add_overlay_from_const_file_decl() {
    let inp = &["const file = 'samples/spam.nu'", "overlay use $file", "foo"];

    let actual = nu!(cwd: "tests/overlays", &inp.join("; "));

    assert_eq!(actual.out, "foo");
}

#[test]
fn add_overlay_from_const_module_name_decl() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "const mod = 'spam'",
        "overlay use $mod",
        "foo",
    ];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "foo");
}

#[test]
fn new_overlay_from_const_name() {
    let inp = &[
        "const mod = 'spam'",
        "overlay new $mod",
        "overlay list | last",
    ];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "spam");
}

#[test]
fn hide_overlay_from_const_name() {
    let inp = &[
        "const mod = 'spam'",
        "overlay new $mod",
        "overlay hide $mod",
        "overlay list | str join ' '",
    ];

    let actual = nu!(&inp.join("; "));

    assert!(!actual.out.contains("spam"));
}

// This one tests that the `nu_repl()` loop works correctly
#[test]
fn add_overlay_from_file_decl_cd() {
    let inp = &["cd samples", "overlay use spam.nu", "foo"];

    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn add_overlay_from_file_alias() {
    let inp = &["overlay use samples/spam.nu", "bar"];

    let actual = nu!(cwd: "tests/overlays", &inp.join("; "));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "bar");
    assert_eq!(actual_repl.out, "bar");
}

#[test]
fn add_overlay_from_file_env() {
    let inp = &["overlay use samples/spam.nu", "$env.BAZ"];

    let actual = nu!(cwd: "tests/overlays", &inp.join("; "));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "baz");
    assert_eq!(actual_repl.out, "baz");
}

#[test]
fn add_overlay_scoped() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "do { overlay use spam }",
        "foo",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(!actual.err.is_empty());
    #[cfg(windows)]
    assert_ne!(actual_repl.out, "foo");
    #[cfg(not(windows))]
    assert!(!actual_repl.err.is_empty());
}

#[test]
fn update_overlay_from_module() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam",
        r#"module spam { export def foo [] { "bar" } }"#,
        "overlay use spam",
        "foo",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "bar");
    assert_eq!(actual_repl.out, "bar");
}

#[test]
fn update_overlay_from_module_env() {
    let inp = &[
        r#"module spam { export-env { $env.FOO = "foo" } }"#,
        "overlay use spam",
        r#"module spam { export-env { $env.FOO = "bar" } }"#,
        "overlay use spam",
        "$env.FOO",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "bar");
    assert_eq!(actual_repl.out, "bar");
}

#[test]
fn overlay_use_do_not_eval_twice() {
    let inp = &[
        r#"module spam { export-env { $env.FOO = "foo" } }"#,
        "overlay use spam",
        r#"$env.FOO = "bar""#,
        "overlay hide spam",
        "overlay use spam",
        "$env.FOO",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "bar");
    assert_eq!(actual_repl.out, "bar");
}

#[test]
fn hide_overlay() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam",
        "overlay hide spam",
        "foo",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(!actual.err.is_empty());
    #[cfg(windows)]
    assert_ne!(actual_repl.out, "foo");
    #[cfg(not(windows))]
    assert!(!actual_repl.err.is_empty());
}

#[test]
fn hide_last_overlay() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam",
        "overlay hide",
        "foo",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(!actual.err.is_empty());
    #[cfg(windows)]
    assert_ne!(actual_repl.out, "foo");
    #[cfg(not(windows))]
    assert!(!actual_repl.err.is_empty());
}

#[test]
fn hide_overlay_scoped() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam",
        "do { overlay hide spam }",
        "foo",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn hide_overlay_env() {
    let inp = &[
        r#"module spam { export-env { $env.FOO = "foo" } }"#,
        "overlay use spam",
        "overlay hide spam",
        "$env.FOO",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(actual.err.contains("not_found"));
    assert!(actual_repl.err.contains("not_found"));
}

#[test]
fn hide_overlay_scoped_env() {
    let inp = &[
        r#"module spam { export-env { $env.FOO = "foo" } }"#,
        "overlay use spam",
        "do { overlay hide spam }",
        "$env.FOO",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn list_default_overlay() {
    let inp = &["overlay list | last"];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "zero");
    assert_eq!(actual_repl.out, "zero");
}

#[test]
fn list_last_overlay() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam",
        "overlay list | last",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "spam");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn list_overlay_scoped() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam",
        "do { overlay list | last }",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "spam");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn hide_overlay_discard_decl() {
    let inp = &[
        "overlay use samples/spam.nu",
        r#"def bagr [] { "bagr" }"#,
        "overlay hide spam",
        "bagr",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(!actual.err.is_empty());
    #[cfg(windows)]
    assert_ne!(actual_repl.out, "bagr");
    #[cfg(not(windows))]
    assert!(!actual_repl.err.is_empty());
}

#[test]
fn hide_overlay_discard_alias() {
    let inp = &[
        "overlay use samples/spam.nu",
        r#"alias bagr = echo "bagr""#,
        "overlay hide spam",
        "bagr",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(!actual.err.is_empty());
    #[cfg(windows)]
    assert_ne!(actual_repl.out, "bagr");
    #[cfg(not(windows))]
    assert!(!actual_repl.err.is_empty());
}

#[test]
fn hide_overlay_discard_env() {
    let inp = &[
        "overlay use samples/spam.nu",
        "$env.BAGR = 'bagr'",
        "overlay hide spam",
        "$env.BAGR",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(actual.err.contains("not_found"));
    assert!(actual_repl.err.contains("not_found"));
}

#[test]
fn hide_overlay_keep_decl() {
    let inp = &[
        "overlay use samples/spam.nu",
        r#"def bagr [] { "bagr" }"#,
        "overlay hide --keep-custom spam",
        "bagr",
    ];

    let actual = nu!(cwd: "tests/overlays", &inp.join("; "));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(actual.out.contains("bagr"));
    assert!(actual_repl.out.contains("bagr"));
}

#[test]
fn hide_overlay_keep_alias() {
    let inp = &[
        "overlay use samples/spam.nu",
        "alias bagr = echo 'bagr'",
        "overlay hide --keep-custom spam",
        "bagr",
    ];

    let actual = nu!(cwd: "tests/overlays", &inp.join("; "));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(actual.out.contains("bagr"));
    assert!(actual_repl.out.contains("bagr"));
}

#[test]
fn hide_overlay_dont_keep_env() {
    let inp = &[
        "overlay use samples/spam.nu",
        "$env.BAGR = 'bagr'",
        "overlay hide --keep-custom spam",
        "$env.BAGR",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(actual.err.contains("not_found"));
    assert!(actual_repl.err.contains("not_found"));
}

#[test]
fn hide_overlay_dont_keep_overwritten_decl() {
    let inp = &[
        "overlay use samples/spam.nu",
        "def foo [] { 'bar' }",
        "overlay hide --keep-custom spam",
        "foo",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(!actual.err.is_empty());
    #[cfg(windows)]
    assert_ne!(actual_repl.out, "bagr");
    #[cfg(not(windows))]
    assert!(!actual_repl.err.is_empty());
}

#[test]
fn hide_overlay_dont_keep_overwritten_alias() {
    let inp = &[
        "overlay use samples/spam.nu",
        "alias bar = echo `baz`",
        "overlay hide --keep-custom spam",
        "bar",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(!actual.err.is_empty());
    #[cfg(windows)]
    assert_ne!(actual_repl.out, "bagr");
    #[cfg(not(windows))]
    assert!(!actual_repl.err.is_empty());
}

#[test]
fn hide_overlay_dont_keep_overwritten_env() {
    let inp = &[
        "overlay use samples/spam.nu",
        "$env.BAZ = 'bagr'",
        "overlay hide --keep-custom spam",
        "$env.BAZ",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(actual.err.contains("not_found"));
    assert!(actual_repl.err.contains("not_found"));
}

#[test]
fn hide_overlay_keep_decl_in_latest_overlay() {
    let inp = &[
        "overlay use samples/spam.nu",
        "def bagr [] { 'bagr' }",
        "module eggs { }",
        "overlay use eggs",
        "overlay hide --keep-custom spam",
        "bagr",
    ];

    let actual = nu!(cwd: "tests/overlays", &inp.join("; "));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(actual.out.contains("bagr"));
    assert!(actual_repl.out.contains("bagr"));
}

#[test]
fn hide_overlay_keep_alias_in_latest_overlay() {
    let inp = &[
        "overlay use samples/spam.nu",
        "alias bagr = echo 'bagr'",
        "module eggs { }",
        "overlay use eggs",
        "overlay hide --keep-custom spam",
        "bagr",
    ];

    let actual = nu!(cwd: "tests/overlays", &inp.join("; "));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert!(actual.out.contains("bagr"));
    assert!(actual_repl.out.contains("bagr"));
}

#[test]
fn hide_overlay_dont_keep_env_in_latest_overlay() {
    let inp = &[
        "overlay use samples/spam.nu",
        "$env.BAGR = 'bagr'",
        "module eggs { }",
        "overlay use eggs",
        "overlay hide --keep-custom spam",
        "$env.BAGR",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(actual.err.contains("not_found"));
    assert!(actual_repl.err.contains("not_found"));
}

#[test]
fn preserve_overrides() {
    let inp = &[
        "overlay use samples/spam.nu",
        r#"def foo [] { "new-foo" }"#,
        "overlay hide spam",
        "overlay use spam",
        "foo",
    ];

    let actual = nu!(cwd: "tests/overlays", &inp.join("; "));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "new-foo");
    assert_eq!(actual_repl.out, "new-foo");
}

#[test]
fn reset_overrides() {
    let inp = &[
        "overlay use samples/spam.nu",
        r#"def foo [] { "new-foo" }"#,
        "overlay hide spam",
        "overlay use samples/spam.nu",
        "foo",
    ];

    let actual = nu!(cwd: "tests/overlays", &inp.join("; "));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn overlay_new() {
    let inp = &["overlay new spam", "overlay list | last"];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "spam");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn overlay_keep_pwd() {
    let inp = &[
        "overlay new spam",
        "cd samples",
        "overlay hide --keep-env [ PWD ] spam",
        "$env.PWD | path basename",
    ];

    let actual = nu!(cwd: "tests/overlays", &inp.join("; "));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "samples");
    assert_eq!(actual_repl.out, "samples");
}

#[test]
fn overlay_reactivate_with_nufile_should_not_change_pwd() {
    let inp = &[
        "overlay use spam.nu",
        "cd ..",
        "overlay hide --keep-env [ PWD ] spam",
        "cd samples",
        "overlay use spam.nu",
        "$env.PWD | path basename",
    ];

    let actual = nu!(cwd: "tests/overlays/samples", &inp.join("; "));
    let actual_repl = nu!(cwd: "tests/overlays/samples", nu_repl_code(inp));

    assert_eq!(actual.out, "samples");
    assert_eq!(actual_repl.out, "samples");
}

#[test]
fn overlay_reactivate_with_module_name_should_change_pwd() {
    let inp = &[
        "overlay use spam.nu",
        "cd ..",
        "overlay hide --keep-env [ PWD ] spam",
        "cd samples",
        "overlay use spam",
        "$env.PWD | path basename",
    ];

    let actual = nu!(cwd: "tests/overlays/samples", &inp.join("; "));
    let actual_repl = nu!(cwd: "tests/overlays/samples", nu_repl_code(inp));

    assert_eq!(actual.out, "overlays");
    assert_eq!(actual_repl.out, "overlays");
}

#[test]
fn overlay_wrong_rename_type() {
    let inp = &["module spam {}", "overlay use spam as { echo foo }"];

    let actual = nu!(&inp.join("; "));

    assert!(actual.err.contains("parse_mismatch"));
}

#[test]
fn overlay_add_renamed() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam as eggs --prefix",
        "eggs foo",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn overlay_add_renamed_const() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "const name = 'spam'",
        "const new_name = 'eggs'",
        "overlay use $name as $new_name --prefix",
        "eggs foo",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn overlay_add_renamed_from_file() {
    let inp = &["overlay use samples/spam.nu as eggs --prefix", "eggs foo"];

    let actual = nu!(cwd: "tests/overlays", &inp.join("; "));
    let actual_repl = nu!(cwd: "tests/overlays", nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn overlay_cant_rename_existing_overlay() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam",
        "overlay hide spam",
        "overlay use spam as eggs",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(actual.err.contains("cant_add_overlay_help"));
    assert!(actual_repl.err.contains("cant_add_overlay_help"));
}

#[test]
fn overlay_can_add_renamed_overlay() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam as eggs --prefix",
        "overlay use spam --prefix",
        "(spam foo) + (eggs foo)",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "foofoo");
    assert_eq!(actual_repl.out, "foofoo");
}

#[test]
fn overlay_hide_renamed_overlay() {
    let inp = &[
        r#"module spam { export def foo-command-which-does-not-conflict [] { "foo" } }"#,
        "overlay use spam as eggs",
        "overlay hide eggs",
        "foo-command-which-does-not-conflict",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(actual.err.contains("external_command"));
    assert!(actual_repl.err.contains("external_command"));
}

#[test]
fn overlay_hide_restore_hidden_env() {
    let inp = &[
        "$env.foo = 'bar'",
        "overlay new aa",
        "hide-env foo",
        "overlay hide aa",
        "$env.foo",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual_repl.out, "bar");
}

#[test]
fn overlay_hide_dont_restore_hidden_env_which_is_introduce_currently() {
    let inp = &[
        "overlay new aa",
        "$env.foo = 'bar'",
        "hide-env foo", // hide the env in overlay `aa`
        "overlay hide aa",
        "'foo' in $env",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual_repl.out, "false");
}

#[test]
fn overlay_hide_and_add_renamed_overlay() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam as eggs",
        "overlay hide eggs",
        "overlay use eggs",
        "foo",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn overlay_use_export_env() {
    let inp = &[
        "module spam { export-env { $env.FOO = 'foo' } }",
        "overlay use spam",
        "$env.FOO",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn overlay_use_export_env_config_affected() {
    let inp = &[
        "mut out = []",
        "$env.config.filesize.unit = 'metric'",
        "$out ++= [(20MB | into string)]",
        "module spam { export-env { $env.config.filesize.unit = 'binary' } }",
        "overlay use spam",
        "$out ++= [(20MiB | into string)]",
        r#"$out | to json --raw"#,
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, r#"["20.0 MB","20.0 MiB"]"#);
    assert_eq!(actual_repl.out, r#"["20.0 MB","20.0 MiB"]"#);
}

#[test]
fn overlay_hide_config_affected() {
    let inp = &[
        "mut out = []",
        "$env.config.filesize.unit = 'metric'",
        "$out ++= [(20MB | into string)]",
        "module spam { export-env { $env.config.filesize.unit = 'binary' } }",
        "overlay use spam",
        "$out ++= [(20MiB | into string)]",
        "overlay hide",
        "$out ++= [(20MB | into string)]",
        r#"$out | to json --raw"#,
    ];

    // Can't hide overlay within the same source file
    // let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    // assert_eq!(actual.out, r#"["20.0 MB","20.0 MiB","20.0 MB"]"#);
    assert_eq!(actual_repl.out, r#"["20.0 MB","20.0 MiB","20.0 MB"]"#);
}

#[test]
fn overlay_use_after_hide_config_affected() {
    let inp = &[
        "mut out = []",
        "$env.config.filesize.unit = 'metric'",
        "$out ++= [(20MB | into string)]",
        "module spam { export-env { $env.config.filesize.unit = 'binary' } }",
        "overlay use spam",
        "$out ++= [(20MiB | into string)]",
        "overlay hide",
        "$out ++= [(20MB | into string)]",
        "overlay use spam",
        "$out ++= [(20MiB | into string)]",
        r#"$out | to json --raw"#,
    ];

    // Can't hide overlay within the same source file
    // let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    // assert_eq!(actual.out, r#"["20.0 MB","20.0 MiB","20.0 MB"]"#);
    assert_eq!(
        actual_repl.out,
        r#"["20.0 MB","20.0 MiB","20.0 MB","20.0 MiB"]"#
    );
}

#[test]
fn overlay_use_export_env_hide() {
    let inp = &[
        "$env.FOO = 'foo'",
        "module spam { export-env { hide-env FOO } }",
        "overlay use spam",
        "$env.FOO",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(actual.err.contains("not_found"));
    assert!(actual_repl.err.contains("not_found"));
}

#[test]
fn overlay_use_do_cd() {
    Playground::setup("overlay_use_do_cd", |dirs, sandbox| {
        sandbox
            .mkdir("test1/test2")
            .with_files(&[FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                "
                    export-env { cd test1/test2 }
                ",
            )]);

        let inp = &[
            "overlay use test1/test2/spam.nu",
            "$env.PWD | path basename",
        ];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert_eq!(actual.out, "test2");
    })
}

#[test]
fn overlay_use_do_cd_file_relative() {
    Playground::setup("overlay_use_do_cd_file_relative", |dirs, sandbox| {
        sandbox
            .mkdir("test1/test2")
            .with_files(&[FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                "
                    export-env { cd ($env.FILE_PWD | path join '..') }
                ",
            )]);

        let inp = &[
            "overlay use test1/test2/spam.nu",
            "$env.PWD | path basename",
        ];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert_eq!(actual.out, "test1");
    })
}

#[test]
fn overlay_use_dont_cd_overlay() {
    Playground::setup("overlay_use_dont_cd_overlay", |dirs, sandbox| {
        sandbox
            .mkdir("test1/test2")
            .with_files(&[FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                "
                    export-env {
                        overlay new spam
                        cd test1/test2
                        overlay hide spam
                    }
                ",
            )]);

        let inp = &["source-env test1/test2/spam.nu", "$env.PWD | path basename"];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert_eq!(actual.out, "overlay_use_dont_cd_overlay");
    })
}

#[test]
fn overlay_use_find_scoped_module() {
    Playground::setup("overlay_use_find_module_scoped", |dirs, _| {
        let inp = "
                do {
                    module spam { }

                    overlay use spam
                    overlay list | last
                }
            ";

        let actual = nu!(cwd: dirs.test(), inp);

        assert_eq!(actual.out, "spam");
    })
}

#[test]
fn overlay_preserve_hidden_env_1() {
    let inp = &[
        "overlay new spam",
        "$env.FOO = 'foo'",
        "overlay new eggs",
        "$env.FOO = 'bar'",
        "hide-env FOO",
        "overlay use eggs",
        "$env.FOO",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn overlay_preserve_hidden_env_2() {
    let inp = &[
        "overlay new spam",
        "$env.FOO = 'foo'",
        "overlay hide spam",
        "overlay new eggs",
        "$env.FOO = 'bar'",
        "hide-env FOO",
        "overlay hide eggs",
        "overlay use spam",
        "overlay use eggs",
        "$env.FOO",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn overlay_reset_hidden_env() {
    let inp = &[
        "overlay new spam",
        "$env.FOO = 'foo'",
        "overlay new eggs",
        "$env.FOO = 'bar'",
        "hide-env FOO",
        "module eggs { export-env { $env.FOO = 'bar' } }",
        "overlay use eggs",
        "$env.FOO",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "bar");
    assert_eq!(actual_repl.out, "bar");
}

#[ignore = "TODO: For this to work, we'd need to make predecls respect overlays"]
#[test]
fn overlay_preserve_hidden_decl() {
    let inp = &[
        "overlay new spam",
        "def foo [] { 'foo' }",
        "overlay new eggs",
        "def foo [] { 'bar' }",
        "hide foo",
        "overlay use eggs",
        "foo",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[ignore = "TODO: For this to work, we'd need to make predecls respect overlays"]
#[test]
fn overlay_preserve_hidden_alias() {
    let inp = &[
        "overlay new spam",
        "alias foo = echo 'foo'",
        "overlay new eggs",
        "alias foo = echo 'bar'",
        "hide foo",
        "overlay use eggs",
        "foo",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn overlay_trim_single_quote() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use 'spam'",
        "overlay list | last ",
    ];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "spam");
}

#[test]
fn overlay_trim_single_quote_hide() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use 'spam'",
        "overlay hide spam ",
        "foo",
    ];
    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(!actual.err.is_empty());
    #[cfg(windows)]
    assert_ne!(actual_repl.out, "foo");
    #[cfg(not(windows))]
    assert!(!actual_repl.err.is_empty());
}

#[test]
fn overlay_trim_double_quote() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use "spam" "#,
        "overlay list | last ",
    ];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "spam");
}

#[test]
fn overlay_trim_double_quote_hide() {
    let inp = &[
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use "spam" "#,
        "overlay hide spam ",
        "foo",
    ];
    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(!actual.err.is_empty());
    #[cfg(windows)]
    assert_ne!(actual_repl.out, "foo");
    #[cfg(not(windows))]
    assert!(!actual_repl.err.is_empty());
}

#[test]
fn overlay_use_and_restore_older_env_vars() {
    let inp = &[
        "module spam {
            export-env {
                let old_baz = $env.BAZ;
                $env.BAZ = $old_baz + 'baz'
            }
        }",
        "$env.BAZ = 'baz'",
        "overlay use spam",
        "overlay hide spam",
        "$env.BAZ = 'new-baz'",
        "overlay use --reload spam",
        "$env.BAZ",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "new-bazbaz");
    assert_eq!(actual_repl.out, "new-bazbaz");
}

#[test]
fn overlay_use_and_reload() {
    let inp = &[
        "module spam {
            export def foo [] { 'foo' };
            export alias fooalias = echo 'foo';
            export-env {
                $env.FOO = 'foo'
            }
        }",
        "overlay use spam",
        "def foo [] { 'newfoo' }",
        "alias fooalias = echo 'newfoo'",
        "$env.FOO = 'newfoo'",
        "overlay use --reload spam",
        "$'(foo)(fooalias)($env.FOO)'",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "foofoofoo");
    assert_eq!(actual_repl.out, "foofoofoo");
}

#[test]
fn overlay_use_and_reolad_keep_custom() {
    let inp = &[
        "overlay new spam",
        "def foo [] { 'newfoo' }",
        "alias fooalias = echo 'newfoo'",
        "$env.FOO = 'newfoo'",
        "overlay use --reload spam",
        "$'(foo)(fooalias)($env.FOO)'",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "newfoonewfoonewfoo");
    assert_eq!(actual_repl.out, "newfoonewfoonewfoo");
}

#[test]
fn overlay_use_main() {
    let inp = &[
        r#"module spam { export def main [] { "spam" } }"#,
        "overlay use spam",
        "spam",
    ];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "spam");
}

#[test]
fn overlay_use_main_prefix() {
    let inp = &[
        r#"module spam { export def main [] { "spam" } }"#,
        "overlay use spam --prefix",
        "spam",
    ];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "spam");
}

#[test]
fn overlay_use_main_def_env() {
    let inp = &[
        r#"module spam { export def --env main [] { $env.SPAM = "spam" } }"#,
        "overlay use spam",
        "spam",
        "$env.SPAM",
    ];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "spam");
}

#[test]
fn overlay_use_main_def_known_external() {
    // note: requires installed cargo
    let inp = &[
        "module cargo { export extern main [] }",
        "overlay use cargo",
        "cargo --version",
    ];

    let actual = nu!(&inp.join("; "));

    assert!(actual.out.contains("cargo"));
}

#[test]
fn overlay_use_main_not_exported() {
    let inp = &[
        r#"module my-super-cool-and-unique-module-name { def main [] { "hi" } }"#,
        "overlay use my-super-cool-and-unique-module-name",
        "my-super-cool-and-unique-module-name",
    ];

    let actual = nu!(&inp.join("; "));

    assert!(actual.err.contains("external_command"));
}

#[test]
fn alias_overlay_hide() {
    let inp = &[
        "overlay new spam",
        "def my-epic-command-name [] { 'foo' }",
        "overlay new eggs",
        "alias oh = overlay hide",
        "oh spam",
        "my-epic-command-name",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(actual.err.contains("external_command"));
    assert!(actual_repl.err.contains("external_command"));
}

#[test]
fn alias_overlay_use() {
    let inp = &[
        "module spam { export def foo [] { 'foo' } }",
        "alias ou = overlay use",
        "ou spam",
        "foo",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "foo");
    assert_eq!(actual_repl.out, "foo");
}

#[test]
fn alias_overlay_use_2() {
    let inp = &[
        "module inner {}",
        "module spam { export alias b = overlay use inner }",
        "use spam",
        "spam b",
        "overlay list | get 1",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(actual.err.is_empty());
    assert!(actual_repl.err.is_empty());
    assert_eq!(actual.out, "inner");
    assert_eq!(actual_repl.out, "inner");
}

#[test]
fn alias_overlay_use_3() {
    let inp = &[
        "module inner {}",
        "module spam { export alias b = overlay use inner }",
        "use spam b",
        "b",
        "overlay list | get 1",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(actual.err.is_empty());
    assert!(actual_repl.err.is_empty());
    assert_eq!(actual.out, "inner");
    assert_eq!(actual_repl.out, "inner");
}

#[test]
fn alias_overlay_new() {
    let inp = &[
        "alias on = overlay new",
        "on spam",
        "on eggs",
        "overlay list | last",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "eggs");
    assert_eq!(actual_repl.out, "eggs");
}

#[test]
fn overlay_new_with_reload() {
    let inp = &[
        "overlay new spam",
        "$env.foo = 'bar'",
        "overlay hide spam",
        "overlay new spam -r",
        "'foo' in $env",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual.out, "false");
    assert_eq!(actual_repl.out, "false");
}

#[test]
fn overlay_use_module_dir() {
    let import = "overlay use samples/spam";

    let inp = &[import, "spam"];
    let actual = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual.out, "spam");

    let inp = &[import, "foo"];
    let actual = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual.out, "foo");

    let inp = &[import, "bar"];
    let actual = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual.out, "bar");

    let inp = &[import, "foo baz"];
    let actual = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual.out, "foobaz");

    let inp = &[import, "bar baz"];
    let actual = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual.out, "barbaz");

    let inp = &[import, "baz"];
    let actual = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual.out, "spambaz");
}

#[test]
fn overlay_use_module_dir_prefix() {
    let import = "overlay use samples/spam --prefix";

    let inp = &[import, "spam"];
    let actual = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual.out, "spam");

    let inp = &[import, "spam foo"];
    let actual = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual.out, "foo");

    let inp = &[import, "spam bar"];
    let actual = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual.out, "bar");

    let inp = &[import, "spam foo baz"];
    let actual = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual.out, "foobaz");

    let inp = &[import, "spam bar baz"];
    let actual = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual.out, "barbaz");

    let inp = &[import, "spam baz"];
    let actual = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual.out, "spambaz");
}

#[test]
fn overlay_help_no_error() {
    let actual = nu!("overlay hide -h");
    assert!(actual.err.is_empty());
    let actual = nu!("overlay new -h");
    assert!(actual.err.is_empty());
    let actual = nu!("overlay use -h");
    assert!(actual.err.is_empty());
}

#[test]
fn test_overlay_use_with_printing_file_pwd() {
    Playground::setup("use_with_printing_file_pwd", |dirs, nu| {
        let file = dirs.test().join("foo").join("mod.nu");
        nu.mkdir("foo").with_files(&[FileWithContent(
            file.as_os_str().to_str().unwrap(),
            r#"
                export-env {
                    print $env.FILE_PWD
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "overlay use foo"
        );

        assert_eq!(actual.out, dirs.test().join("foo").to_string_lossy());
    });
}

#[test]
fn test_overlay_use_with_printing_current_file() {
    Playground::setup("use_with_printing_current_file", |dirs, nu| {
        let file = dirs.test().join("foo").join("mod.nu");
        nu.mkdir("foo").with_files(&[FileWithContent(
            file.as_os_str().to_str().unwrap(),
            r#"
                export-env {
                    print $env.CURRENT_FILE
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "overlay use foo"
        );

        assert_eq!(
            actual.out,
            dirs.test().join("foo").join("mod.nu").to_string_lossy()
        );
    });
}

#[test]
fn report_errors_in_export_env() {
    let inp = &[
        r#"module spam { export-env { error make -u {msg: "reported"} } }"#,
        "overlay use spam",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(actual.err.contains("reported"));
    assert!(actual_repl.err.contains("reported"));
}
