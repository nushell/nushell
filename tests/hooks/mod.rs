use super::nu_repl::nu_repl;

fn hook_env_with_code(code: &str) -> String {
    format!(
        r#"let-env config = {{
            hooks: {{
                env_change_str: {{
                    FOO : [
                        {{
                            code: {code}
                        }}
                    ]
                }}
            }}
        }}"#
    )
}

#[test]
fn env_change_define_command() {
    let inp = &[
        &hook_env_with_code(r#"'def foo [] { "got foo!" }'"#),
        "let-env FOO = 1",
        "foo",
    ];

    let actual_repl = nu_repl("tests/hooks", inp);

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "got foo!");
}

#[test]
fn env_change_define_variable() {
    let inp = &[
        &hook_env_with_code(r#"'let x = "spam"'"#),
        "let-env FOO = 1",
        "$x",
    ];

    let actual_repl = nu_repl("tests/hooks", inp);

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn env_change_define_env_var() {
    let inp = &[
        &hook_env_with_code(r#"'let-env SPAM = "spam"'"#),
        "let-env FOO = 1",
        "$env.SPAM",
    ];

    let actual_repl = nu_repl("tests/hooks", inp);

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn env_change_define_alias() {
    let inp = &[
        &hook_env_with_code(r#"'alias spam = "spam"'"#),
        "let-env FOO = 1",
        "spam",
    ];

    let actual_repl = nu_repl("tests/hooks", inp);

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "spam");
}
