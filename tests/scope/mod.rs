use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::nu;
use nu_test_support::playground::Playground;
use pretty_assertions::assert_eq;

// Note: These tests might slightly overlap with crates/nu-command/tests/commands/help.rs

#[test]
fn scope_shows_alias() {
    let actual = nu!("alias xaz = echo alias1
        scope aliases | find xaz | length
        ");

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 1);
}

#[test]
fn scope_shows_command() {
    let actual = nu!("def xaz [] { echo xaz }
        scope commands | find xaz | length
        ");

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 1);
}

#[test]
fn scope_doesnt_show_scoped_hidden_alias() {
    let actual = nu!("alias xaz = echo alias1
        do {
            hide xaz
            scope aliases | find xaz | length
        }
        ");

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 0);
}

#[test]
fn scope_doesnt_show_hidden_alias() {
    let actual = nu!("alias xaz = echo alias1
        hide xaz
        scope aliases | find xaz | length
        ");

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 0);
}

#[test]
fn scope_doesnt_show_scoped_hidden_command() {
    let actual = nu!("def xaz [] { echo xaz }
        do {
            hide xaz
            scope commands | find xaz | length
        }
        ");

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 0);
}

#[test]
fn scope_doesnt_show_hidden_command() {
    let actual = nu!("def xaz [] { echo xaz }
        hide xaz
        scope commands | find xaz | length
        ");

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 0);
}

// same problem as 'which' command
#[ignore = "See https://github.com/nushell/nushell/issues/4837"]
#[test]
fn correctly_report_of_shadowed_alias() {
    let actual = nu!("alias xaz = echo alias1
        def helper [] {
            alias xaz = echo alias2
            scope aliases
        }
        helper | where alias == xaz | get expansion.0");

    assert_eq!(actual.out, "echo alias2");
}

#[test]
fn correct_scope_modules_fields() {
    let module_setup = r#"
        # nice spam
        #
        # and some extra description for spam

        export module eggs {
            export module bacon {
                export def sausage [] { 'sausage' }
            }
        }

        export def main [] { 'foo' };
        export alias xaz = print
        export extern git []
        export const X = 4

        export-env { $env.SPAM = 'spam' }
    "#;

    Playground::setup("correct_scope_modules_fields", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("spam.nu", module_setup)]);

        let inp = &[
            "use spam.nu",
            "scope modules | where name == spam | get 0.name",
        ];
        let actual = nu!(cwd: dirs.test(), &inp.join("; "));
        assert_eq!(actual.out, "spam");

        let inp = &[
            "use spam.nu",
            "scope modules | where name == spam | get 0.description",
        ];
        let actual = nu!(cwd: dirs.test(), &inp.join("; "));
        assert_eq!(actual.out, "nice spam");

        let inp = &[
            "use spam.nu",
            "scope modules | where name == spam | get 0.extra_description",
        ];
        let actual = nu!(cwd: dirs.test(), &inp.join("; "));
        assert_eq!(actual.out, "and some extra description for spam");

        let inp = &[
            "use spam.nu",
            "scope modules | where name == spam | get 0.has_env_block",
        ];
        let actual = nu!(cwd: dirs.test(), &inp.join("; "));
        assert_eq!(actual.out, "true");

        let inp = &[
            "use spam.nu",
            "scope modules | where name == spam | get 0.commands.0.name",
        ];
        let actual = nu!(cwd: dirs.test(), &inp.join("; "));
        assert_eq!(actual.out, "spam");

        let inp = &[
            "use spam.nu",
            "scope modules | where name == spam | get 0.aliases.0.name",
        ];
        let actual = nu!(cwd: dirs.test(), &inp.join("; "));
        assert_eq!(actual.out, "xaz");

        let inp = &[
            "use spam.nu",
            "scope modules | where name == spam | get 0.externs.0.name",
        ];
        let actual = nu!(cwd: dirs.test(), &inp.join("; "));
        assert_eq!(actual.out, "git");

        let inp = &[
            "use spam.nu",
            "scope modules | where name == spam | get 0.constants.0.name",
        ];
        let actual = nu!(cwd: dirs.test(), &inp.join("; "));
        assert_eq!(actual.out, "X");

        let inp = &[
            "use spam.nu",
            "scope modules | where name == spam | get 0.submodules.0.submodules.0.name",
        ];
        let actual = nu!(cwd: dirs.test(), &inp.join("; "));
        assert_eq!(actual.out, "bacon");

        let inp = &[
            "use spam.nu",
            "scope modules | where name == spam | get 0.submodules.0.submodules.0.commands.0.name",
        ];
        let actual = nu!(cwd: dirs.test(), &inp.join("; "));
        assert_eq!(actual.out, "sausage");
    })
}

#[test]
fn correct_scope_aliases_fields() {
    let module_setup = r#"
        # nice alias
        export alias xaz = print
    "#;

    Playground::setup("correct_scope_aliases_fields", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("spam.nu", module_setup)]);

        let inp = &[
            "use spam.nu",
            "scope aliases | where name == 'spam xaz' | get 0.name",
        ];
        let actual = nu!(cwd: dirs.test(), &inp.join("; "));
        assert_eq!(actual.out, "spam xaz");

        let inp = &[
            "use spam.nu",
            "scope aliases | where name == 'spam xaz' | get 0.expansion",
        ];
        let actual = nu!(cwd: dirs.test(), &inp.join("; "));
        assert_eq!(actual.out, "print");

        let inp = &[
            "use spam.nu",
            "scope aliases | where name == 'spam xaz' | get 0.description",
        ];
        let actual = nu!(cwd: dirs.test(), &inp.join("; "));
        assert_eq!(actual.out, "nice alias");

        let inp = &[
            "use spam.nu",
            "scope aliases | where name == 'spam xaz' | get 0.decl_id | is-empty",
        ];
        let actual = nu!(cwd: dirs.test(), &inp.join("; "));
        assert_eq!(actual.out, "false");

        let inp = &[
            "use spam.nu",
            "scope aliases | where name == 'spam xaz' | get 0.aliased_decl_id | is-empty",
        ];
        let actual = nu!(cwd: dirs.test(), &inp.join("; "));
        assert_eq!(actual.out, "false");
    })
}

#[test]
fn scope_alias_aliased_decl_id_external() {
    let inp = &[
        "alias c = cargo",
        "scope aliases | where name == c | get 0.aliased_decl_id | is-empty",
    ];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "true");
}

#[test]
fn correct_scope_externs_fields() {
    let module_setup = r#"
        # nice extern
        export extern git []
    "#;

    Playground::setup("correct_scope_aliases_fields", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("spam.nu", module_setup)]);

        let inp = &[
            "use spam.nu",
            "scope externs | where name == 'spam git' | get 0.name",
        ];
        let actual = nu!(cwd: dirs.test(), &inp.join("; "));
        assert_eq!(actual.out, "spam git");

        let inp = &[
            "use spam.nu",
            "scope externs | where name == 'spam git' | get 0.description",
        ];
        let actual = nu!(cwd: dirs.test(), &inp.join("; "));
        assert_eq!(actual.out, "nice extern");

        let inp = &[
            "use spam.nu",
            "scope externs | where name == 'spam git' | get 0.description | str contains (char nl)",
        ];
        let actual = nu!(cwd: dirs.test(), &inp.join("; "));
        assert_eq!(actual.out, "false");

        let inp = &[
            "use spam.nu",
            "scope externs | where name == 'spam git' | get 0.decl_id | is-empty",
        ];
        let actual = nu!(cwd: dirs.test(), &inp.join("; "));
        assert_eq!(actual.out, "false");
    })
}

#[test]
fn scope_externs_sorted() {
    let inp = &[
        "extern a []",
        "extern b []",
        "extern c []",
        "scope externs | get name | str join ''",
    ];

    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "abc");
}

#[test]
fn correct_scope_variables_fields() {
    let inp = &[
        "let x = 'x'",
        "scope variables | where name == '$x' | get 0.type",
    ];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "string");

    let inp = &[
        "let x = 'x'",
        "scope variables | where name == '$x' | get 0.value",
    ];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "x");

    let inp = &[
        "let x = 'x'",
        "scope variables | where name == '$x' | get 0.is_const",
    ];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "false");

    let inp = &[
        "const x = 'x'",
        "scope variables | where name == '$x' | get 0.is_const",
    ];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "true");

    let inp = &[
        "let x = 'x'",
        "scope variables | where name == '$x' | get 0.var_id | is-empty",
    ];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "false");
}

#[test]
fn example_results_have_valid_span() {
    let inp = &[
        "scope commands",
        "| where name == 'do'",
        "| first",
        "| get examples",
        "| where result == 177",
        "| get 0.result",
        "| metadata",
        "| view span $in.span.start $in.span.end",
    ];
    let actual = nu!(&inp.join(" "));
    assert_eq!(actual.out, "scope commands");
}
