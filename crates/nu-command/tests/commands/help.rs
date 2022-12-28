use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, nu_repl_code, pipeline};

#[test]
fn help_commands_length() {
    let actual = nu!(
    cwd: ".", pipeline(
        r#"
        help commands | length
        "#
    ));

    let output = actual.out;
    let output_int: i32 = output.parse().unwrap();
    let is_positive = output_int.is_positive();
    assert!(is_positive);
}

#[test]
fn help_shows_signature() {
    let actual = nu!(cwd: ".", pipeline("help str distance"));
    assert!(actual
        .out
        .contains("<string> | str distance <string> -> <int>"));

    // don't show signature for parser keyword
    let actual = nu!(cwd: ".", pipeline("help alias"));
    assert!(!actual.out.contains("Signatures"));
}

#[test]
fn help_aliases() {
    let code = &[
        "alias SPAM = print 'spam'",
        "help aliases | where name == SPAM | length",
    ];
    let actual = nu!(cwd: ".", nu_repl_code(code));

    assert_eq!(actual.out, "1");
}

#[test]
fn help_alias_usage_1() {
    Playground::setup("help_alias_usage_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "spam.nu",
            r#"
                # line1
                alias SPAM = print 'spam'
            "#,
        )]);

        let code = &[
            "source spam.nu",
            "help aliases | where name == SPAM | get 0.usage",
        ];
        let actual = nu!(cwd: dirs.test(), nu_repl_code(code));

        assert_eq!(actual.out, "line1");
    })
}

#[test]
fn help_alias_usage_2() {
    let code = &[
        "alias SPAM = print 'spam'  # line2",
        "help aliases | where name == SPAM | get 0.usage",
    ];
    let actual = nu!(cwd: ".", nu_repl_code(code));

    assert_eq!(actual.out, "line2");
}

#[test]
fn help_alias_usage_3() {
    Playground::setup("help_alias_usage_3", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "spam.nu",
            r#"
                # line1
                alias SPAM = print 'spam' # line2
            "#,
        )]);

        let code = &[
            "source spam.nu",
            "help aliases | where name == SPAM | get 0.usage",
        ];
        let actual = nu!(cwd: dirs.test(), nu_repl_code(code));

        assert!(actual.out.contains("line1"));
        assert!(actual.out.contains("line2"));
    })
}

#[test]
fn help_alias_name() {
    Playground::setup("help_alias_name", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "spam.nu",
            r#"
                # line1
                alias SPAM = print 'spam' # line2
            "#,
        )]);

        let code = &["source spam.nu", "help aliases SPAM"];
        let actual = nu!(cwd: dirs.test(), nu_repl_code(code));

        assert!(actual.out.contains("line1"));
        assert!(actual.out.contains("line2"));
        assert!(actual.out.contains("SPAM"));
        assert!(actual.out.contains("print 'spam'"));
    })
}

#[test]
fn help_alias_name_f() {
    Playground::setup("help_alias_name_f", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "spam.nu",
            r#"
                # line1
                alias SPAM = print 'spam' # line2
            "#,
        )]);

        let code = &["source spam.nu", "help aliases -f SPAM | get 0.usage"];
        let actual = nu!(cwd: dirs.test(), nu_repl_code(code));

        assert!(actual.out.contains("line1"));
        assert!(actual.out.contains("line2"));
    })
}

#[test]
fn help_export_alias_name_single_word() {
    Playground::setup("help_export_alias_name_single_word", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "spam.nu",
            r#"
                # line1
                export alias SPAM = print 'spam' # line2
            "#,
        )]);

        let code = &["use spam.nu SPAM", "help aliases SPAM"];
        let actual = nu!(cwd: dirs.test(), nu_repl_code(code));

        assert!(actual.out.contains("line1"));
        assert!(actual.out.contains("line2"));
        assert!(actual.out.contains("SPAM"));
        assert!(actual.out.contains("print 'spam'"));
    })
}

#[test]
fn help_export_alias_name_multi_word() {
    Playground::setup("help_export_alias_name_multi_word", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "spam.nu",
            r#"
                # line1
                export alias SPAM = print 'spam' # line2
            "#,
        )]);

        let code = &["use spam.nu", "help aliases spam SPAM"];
        let actual = nu!(cwd: dirs.test(), nu_repl_code(code));

        assert!(actual.out.contains("line1"));
        assert!(actual.out.contains("line2"));
        assert!(actual.out.contains("SPAM"));
        assert!(actual.out.contains("print 'spam'"));
    })
}
