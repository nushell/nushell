use nu_test_support::{nu, nu_repl_code};
use pretty_assertions::assert_eq;

fn env_change_hook_code_list(name: &str, code_list: &[&str]) -> String {
    let mut list = String::new();

    for code in code_list.iter() {
        list.push_str("{ code: ");
        list.push_str(code);
        list.push_str(" }\n");
    }

    format!(
        "$env.config = {{
            hooks: {{
                env_change: {{
                    {name} : [
                        {list}
                    ]
                }}
            }}
        }}"
    )
}

fn env_change_hook(name: &str, code: &str) -> String {
    format!(
        "$env.config = {{
            hooks: {{
                env_change: {{
                    {name}: [{code}]
                }}
            }}
        }}"
    )
}

fn env_change_hook_code(name: &str, code: &str) -> String {
    format!(
        "$env.config = {{
            hooks: {{
                env_change: {{
                    {name}: [{{
                        code: {code}
                    }}]
                }}
            }}
        }}"
    )
}

fn env_change_hook_code_condition(name: &str, condition: &str, code: &str) -> String {
    format!(
        "$env.config = {{
            hooks: {{
                env_change: {{
                    {name}: [{{
                        condition: {condition}
                        code: {code}
                    }}]
                }}
            }}
        }}"
    )
}

fn pre_prompt_hook(code: &str) -> String {
    format!(
        "$env.config = {{
            hooks: {{
                pre_prompt: [{code}]
            }}
        }}"
    )
}

fn pre_prompt_hook_code(code: &str) -> String {
    format!(
        "$env.config = {{
            hooks: {{
                pre_prompt: [{{
                    code: {code}
                }}]
            }}
        }}"
    )
}

fn pre_execution_hook(code: &str) -> String {
    format!(
        "$env.config = {{
            hooks: {{
                pre_execution: [{code}]
            }}
        }}"
    )
}

fn pre_execution_hook_code(code: &str) -> String {
    format!(
        "$env.config = {{
            hooks: {{
                pre_execution: [{{
                    code: {code}
                }}]
            }}
        }}"
    )
}

#[test]
fn env_change_define_command() {
    let inp = &[
        &env_change_hook_code("FOO", r#"'def foo [] { "got foo!" }'"#),
        "$env.FOO = 1",
        "foo",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "got foo!");
}

#[test]
fn env_change_define_variable() {
    let inp = &[
        &env_change_hook_code("FOO", r#"'let x = "spam"'"#),
        "$env.FOO = 1",
        "$x",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn env_change_define_env_var() {
    let inp = &[
        &env_change_hook_code("FOO", r#"'$env.SPAM = "spam"'"#),
        "$env.FOO = 1",
        "$env.SPAM",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn env_change_define_alias() {
    let inp = &[
        &env_change_hook_code("FOO", r#"'alias spam = echo "spam"'"#),
        "$env.FOO = 1",
        "spam",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn env_change_simple_block_preserve_env_var() {
    let inp = &[
        &env_change_hook("FOO", r#"{|| $env.SPAM = "spam" }"#),
        "$env.FOO = 1",
        "$env.SPAM",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn env_change_simple_block_list_shadow_env_var() {
    let inp = &[
        &env_change_hook(
            "FOO",
            r#"[
                {|| $env.SPAM = "foo" }
                {|| $env.SPAM = "spam" }
            ]"#,
        ),
        "$env.FOO = 1",
        "$env.SPAM",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn env_change_block_preserve_env_var() {
    let inp = &[
        &env_change_hook_code("FOO", r#"{|| $env.SPAM = "spam" }"#),
        "$env.FOO = 1",
        "$env.SPAM",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn pre_prompt_define_command() {
    let inp = &[
        &pre_prompt_hook_code(r#"'def foo [] { "got foo!" }'"#),
        "foo",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "got foo!");
}

#[test]
fn pre_prompt_simple_block_preserve_env_var() {
    let inp = &[&pre_prompt_hook(r#"{|| $env.SPAM = "spam" }"#), "$env.SPAM"];

    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn pre_prompt_simple_block_list_shadow_env_var() {
    let inp = &[
        &pre_prompt_hook(
            r#"[
                {|| $env.SPAM = "foo" }
                {|| $env.SPAM = "spam" }
            ]"#,
        ),
        "$env.SPAM",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn pre_prompt_block_preserve_env_var() {
    let inp = &[
        &pre_prompt_hook_code(r#"{|| $env.SPAM = "spam" }"#),
        "$env.SPAM",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn pre_execution_define_command() {
    let inp = &[
        &pre_execution_hook_code(r#"'def foo [] { "got foo!" }'"#),
        "foo",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "got foo!");
}

#[test]
fn pre_execution_simple_block_preserve_env_var() {
    let inp = &[
        &pre_execution_hook(r#"{|| $env.SPAM = "spam" }"#),
        "$env.SPAM",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn pre_execution_simple_block_list_shadow_env_var() {
    let inp = &[
        &pre_execution_hook(
            r#"[
            {|| $env.SPAM = "foo" }
            {|| $env.SPAM = "spam" }
        ]"#,
        ),
        "$env.SPAM",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn pre_execution_block_preserve_env_var() {
    let inp = &[
        &pre_execution_hook_code(r#"{|| $env.SPAM = "spam" }"#),
        "$env.SPAM",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn pre_execution_commandline() {
    let inp = &[
        &pre_execution_hook_code("{|| $env.repl_commandline = (commandline) }"),
        "$env.repl_commandline",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "$env.repl_commandline");
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
        "$env.FOO = 1",
        "foo",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "got foo!");
}

#[test]
fn env_change_block_dont_preserve_command() {
    let inp = &[
        &env_change_hook_code("FOO", r#"{|| def foo [] { "foo" } }"#),
        "$env.FOO = 1",
        "foo",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    #[cfg(windows)]
    assert_ne!(actual_repl.out, "foo");
    #[cfg(not(windows))]
    assert!(actual_repl.err.contains("external_command"));
}

#[test]
fn env_change_block_condition_pwd() {
    let inp = &[
        &env_change_hook_code_condition(
            "PWD",
            "{|before, after| ($after | path basename) == samples }",
            "'source-env .nu-env'",
        ),
        "cd samples",
        "$env.SPAM",
    ];

    let actual_repl = nu!(cwd: "tests/hooks", nu_repl_code(inp));

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn env_change_block_condition_correct_args() {
    let inp = &[
        "$env.FOO = 1",
        &env_change_hook_code_condition(
            "FOO",
            "{|before, after| $before == 1 and $after == 2}",
            "{|before, after| $env.SPAM = ($before == 1 and $after == 2) }",
        ),
        "",
        "$env.FOO = 2",
        "$env.SPAM",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "true");
}

#[test]
fn env_change_dont_panic_with_many_args() {
    let inp = &[
        &env_change_hook_code("FOO", "{ |a, b, c| $env.SPAM = 'spam' }"),
        "$env.FOO = 1",
        "",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert!(actual_repl.err.contains("incompatible_parameters"));
    assert_eq!(actual_repl.out, "");
}

#[test]
fn err_hook_wrong_env_type_1() {
    let inp = &[
        "$env.config = {
            hooks: {
                env_change: {
                    FOO : 1
                }
            }
        }",
        "$env.FOO = 1",
        "",
    ];

    let actual_repl = nu!(nu_repl_code(inp));
    dbg!(&actual_repl.err);

    assert!(actual_repl.err.contains("Type mismatch"));
    assert_eq!(actual_repl.out, "");
}

#[test]
fn err_hook_wrong_env_type_2() {
    let inp = &[
        r#"$env.config = {
            hooks: {
                env_change: "print spam"
            }
        }"#,
        "",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert!(actual_repl.err.contains("Type mismatch"));
    assert_eq!(actual_repl.out, "");
}

#[test]
fn err_hook_wrong_env_type_3() {
    let inp = &[
        "$env.config = {
            hooks: {
                env_change: {
                    FOO : {
                        code: 1
                    }
                }
            }
        }",
        "$env.FOO = 1",
        "",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert!(actual_repl.err.contains("Type mismatch"));
    assert_eq!(actual_repl.out, "");
}

#[test]
fn err_hook_non_boolean_condition_output() {
    let inp = &[
        r#"$env.config = {
            hooks: {
                env_change: {
                    FOO : {
                        condition: {|| "foo" }
                        code: "print spam"
                    }
                }
            }
        }"#,
        "$env.FOO = 1",
        "",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert!(actual_repl.err.contains("Type mismatch"));
    assert_eq!(actual_repl.out, "");
}

#[test]
fn err_hook_non_condition_not_a_block() {
    let inp = &[
        r#"$env.config = {
            hooks: {
                env_change: {
                    FOO : {
                        condition: "foo"
                        code: "print spam"
                    }
                }
            }
        }"#,
        "$env.FOO = 1",
        "",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert!(actual_repl.err.contains("Type mismatch"));
    assert_eq!(actual_repl.out, "");
}

#[test]
fn err_hook_parse_error() {
    let inp = &[
        r#"$env.config = {
            hooks: {
                env_change: {
                    FOO: [{
                        code: "def foo { 'foo' }"
                    }]
                }
            }
        }"#,
        "$env.FOO = 1",
        "",
    ];

    let actual_repl = nu!(nu_repl_code(inp));

    assert!(actual_repl.err.contains("source code has errors"));
    assert_eq!(actual_repl.out, "");
}

#[test]
fn env_change_overlay() {
    let inp = &[
        "module test { export-env { $env.BAR = 2 } }",
        &env_change_hook_code("FOO", "'overlay use test'"),
        "$env.FOO = 1",
        "$env.BAR",
    ];

    let actual_repl = nu!(nu_repl_code(inp));
    assert_eq!(actual_repl.out, "2");
}
