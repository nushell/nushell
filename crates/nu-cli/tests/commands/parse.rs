use nu_test_support::fs::Stub;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

mod simple {
    use super::*;

    #[test]
    fn extracts_fields_from_the_given_the_pattern() {
        Playground::setup("parse_test_1", |dirs, sandbox| {
            sandbox.with_files(vec![Stub::FileWithContentToBeTrimmed(
                "key_value_separated_arepa_ingredients.txt",
                r#"
                    VAR1=Cheese
                    VAR2=JonathanParsed
                    VAR3=NushellSecretIngredient
                "#,
            )]);

            let actual = nu!(
                cwd: dirs.test(), pipeline(
                r#"
                    open key_value_separated_arepa_ingredients.txt
                    | lines
                    | each { echo $it | parse "{Name}={Value}" }
                    | nth 1
                    | echo $it.Value
                "#
            ));

            assert_eq!(actual.out, "JonathanParsed");
        })
    }

    #[test]
    fn double_open_curly_evalutes_to_a_single_curly() {
        Playground::setup("parse_test_regex_2", |dirs, _sandbox| {
            let actual = nu!(
                cwd: dirs.test(), pipeline(
                r#"
                echo "{abc}123"
                | parse "{{abc}{name}"
                | echo $it.name
            "#
            ));

            assert_eq!(actual.out, "123");
        })
    }

    #[test]
    fn properly_escapes_text() {
        Playground::setup("parse_test_regex_3", |dirs, _sandbox| {
            let actual = nu!(
                cwd: dirs.test(), pipeline(
                r#"
                echo "(abc)123"
                | parse "(abc){name}"
                | echo $it.name
            "#
            ));

            assert_eq!(actual.out, "123");
        })
    }
}

mod regex {
    use super::*;

    fn nushell_git_log_oneline<'a>() -> Vec<Stub<'a>> {
        vec![Stub::FileWithContentToBeTrimmed(
            "nushell_git_log_oneline.txt",
            r#"
                ae87582c Fix missing invocation errors (#1846)
                b89976da let format access variables also (#1842)
            "#,
        )]
    }

    #[test]
    fn extracts_fields_with_all_named_groups() {
        Playground::setup("parse_test_regex_1", |dirs, sandbox| {
            sandbox.with_files(nushell_git_log_oneline());

            let actual = nu!(
                cwd: dirs.test(), pipeline(
                r#"
                open nushell_git_log_oneline.txt
                | parse --regex "(?P<Hash>\w+) (?P<Message>.+) \(#(?P<PR>\d+)\)"
                | nth 1
                | get PR
                | echo $it
            "#
            ));

            assert_eq!(actual.out, "1842");
        })
    }

    #[test]
    fn extracts_fields_with_all_unnamed_groups() {
        Playground::setup("parse_test_regex_2", |dirs, sandbox| {
            sandbox.with_files(nushell_git_log_oneline());

            let actual = nu!(
                cwd: dirs.test(), pipeline(
                r#"
                open nushell_git_log_oneline.txt
                | parse --regex "(\w+) (.+) \(#(\d+)\)"
                | nth 1
                | get Capture1
                | echo $it
            "#
            ));

            assert_eq!(actual.out, "b89976da");
        })
    }

    #[test]
    fn extracts_fields_with_named_and_unnamed_groups() {
        Playground::setup("parse_test_regex_3", |dirs, sandbox| {
            sandbox.with_files(nushell_git_log_oneline());

            let actual = nu!(
                cwd: dirs.test(), pipeline(
                r#"
                open nushell_git_log_oneline.txt
                | parse --regex "(?P<Hash>\w+) (.+) \(#(?P<PR>\d+)\)"
                | nth 1
                | get Capture2
                | echo $it
            "#
            ));

            assert_eq!(actual.out, "let format access variables also");
        })
    }
}
