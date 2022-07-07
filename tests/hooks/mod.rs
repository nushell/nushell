use super::nu_repl::nu_repl;

fn hooks_setup_code() -> &'static str {
    r#"let-env config = {
        hooks: {
            env_change_str: {
                FOO : [
                    {
                        code: 'def foo [] { "got foo!" }'
                    }
                ]
            }
        }
    }"#
}

#[test]
fn env_change_define_command() {
    let inp = &[hooks_setup_code(), "let-env FOO = 1", "foo"];

    let actual_repl = nu_repl("tests/hooks", inp);

    assert_eq!(actual_repl.err, "");
    assert_eq!(actual_repl.out, "got foo!");
}
