use super::nu_repl::nu_repl;

fn env_change_hook_code_list(name: &str, code_list: &[&str]) -> String {
    let mut list = String::new();

    for code in code_list.iter() {
        list.push_str("{ code: ");
        list.push_str(code);
        list.push_str(" }\n");
    }

    format!(
        r#"let-env config = {{
            hooks: {{
                env_change: {{
                    {name} : [
                        {list}
                    ]
                }}
            }}
        }}"#
    )
}

fn env_change_hook_code(name: &str, code: &str) -> String {
    format!(
        r#"let-env config = {{
            hooks: {{
                env_change: {{
                    {name} : {{
                        code: {code}
                    }}
                }}
            }}
        }}"#
    )
}

fn env_change_hook_code_condition(name: &str, condition: &str, code: &str) -> String {
    format!(
        r#"let-env config = {{
            hooks: {{
                env_change: {{
                    {name} : {{
                        condition: {condition}
                        code: {code}
                    }}
                }}
            }}
        }}"#
    )
}

fn pre_prompt_hook_code(code: &str) -> String {
    format!(
        r#"let-env config = {{
            hooks: {{
                pre_prompt: {{
                    code: {code}
                }}
            }}
        }}"#
    )
}

fn pre_execution_hook_code(code: &str) -> String {
    format!(
        r#"let-env config = {{
            hooks: {{
                pre_execution: {{
                    code: {code}
                }}
            }}
        }}"#
    )
}

#[test]
fn env_change_define_command() {
    let inp = &[
        &env_change_hook_code("FOO", r#"'def foo [] { "got foo!" }'"#),
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
        &env_change_hook_code("FOO", r#"'let x = "spam"'"#),
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
        &env_change_hook_code("FOO", r#"'let-env SPAM = "spam"'"#),
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
        &env_change_hook_code("FOO", r#"'alias spam = "spam"'"#),
        "let-env FOO = 1",
        "spam",
    ];

    let actual_repl = nu_repl("tests/hooks", inp);

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn env_change_block_preserve_env_var() {
    let inp = &[
        &env_change_hook_code("FOO", r#"{ let-env SPAM = "spam" }"#),
        "let-env FOO = 1",
        "$env.SPAM",
    ];

    let actual_repl = nu_repl("tests/hooks", inp);

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn pre_prompt_define_command() {
    let inp = &[
        &pre_prompt_hook_code(r#"'def foo [] { "got foo!" }'"#),
        "",
        "foo",
    ];

    let actual_repl = nu_repl("tests/hooks", inp);

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "got foo!");
}

#[test]
fn pre_prompt_block_preserve_env_var() {
    let inp = &[
        &pre_prompt_hook_code(r#"{ let-env SPAM = "spam" }"#),
        "",
        "$env.SPAM",
    ];

    let actual_repl = nu_repl("tests/hooks", inp);

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn pre_execution_define_command() {
    let inp = &[
        &pre_execution_hook_code(r#"'def foo [] { "got foo!" }'"#),
        "",
        "foo",
    ];

    let actual_repl = nu_repl("tests/hooks", inp);

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "got foo!");
}

#[test]
fn pre_execution_block_preserve_env_var() {
    let inp = &[
        &pre_execution_hook_code(r#"{ let-env SPAM = "spam" }"#),
        "",
        "$env.SPAM",
    ];

    let actual_repl = nu_repl("tests/hooks", inp);

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn env_change_shadow_command() {
    let inp = &[
        &env_change_hook_code_list(
            "FOO",
            &[
                r#"'def foo [] { "got spam!" }'"#,
                r#"'def foo [] { "got foo!" }'"#,
            ],
        ),
        "let-env FOO = 1",
        "foo",
    ];

    let actual_repl = nu_repl("tests/hooks", inp);

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "got foo!");
}

#[test]
fn env_change_block_dont_preserve_command() {
    let inp = &[
        &env_change_hook_code("FOO", r#"{ def foo [] { "foo" } }"#),
        "let-env FOO = 1",
        "foo",
    ];

    let actual_repl = nu_repl("tests/hooks", inp);

    assert!(!actual_repl.err.is_empty());
    assert!(actual_repl.out.is_empty());
}

#[test]
fn env_change_block_condition_pwd() {
    let inp = &[
        &env_change_hook_code_condition(
            "PWD",
            r#"{ |before, after| ($after | path basename) == samples }"#,
            r#"{ let-env SPAM = "spam" }"#,
        ),
        "cd samples",
        "$env.SPAM",
    ];

    let actual_repl = nu_repl("tests/hooks", inp);

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn env_change_dont_panic_with_many_args() {
    let inp = &[
        &env_change_hook_code("FOO", r#"{ |a, b, c| let-env SPAM = 'spam' }"#),
        "let-env FOO = 1",
        "",
    ];

    let actual_repl = nu_repl("tests/hooks", inp);

    assert!(actual_repl.err.contains("IncompatibleParametersSingle"));
    assert_eq!(actual_repl.out, "");
}
