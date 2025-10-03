use nu_test_support::fs::Stub;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

mod simple {
    use super::*;

    #[test]
    fn extracts_fields_from_the_given_the_pattern() {
        Playground::setup("parse_test_simple_1", |dirs, sandbox| {
            sandbox.with_files(&[Stub::FileWithContentToBeTrimmed(
                "key_value_separated_arepa_ingredients.txt",
                r#"
                    VAR1=Cheese
                    VAR2=JTParsed
                    VAR3=NushellSecretIngredient
                "#,
            )]);

            let actual = nu!(cwd: dirs.test(), r#"
                open key_value_separated_arepa_ingredients.txt
                | lines
                | each { |it| echo $it | parse "{Name}={Value}" }
                | flatten
                | get 1
                | get Value
            "#);

            assert_eq!(actual.out, "JTParsed");
        })
    }

    #[test]
    fn double_open_curly_evaluates_to_a_single_curly() {
        let actual = nu!(r#"
            echo "{abc}123"
            | parse "{{abc}{name}"
            | get name.0
        "#);
        assert_eq!(actual.out, "123");
    }

    #[test]
    fn properly_escapes_text() {
        let actual = nu!(r#"
            echo "(abc)123"
            | parse "(abc){name}"
            | get name.0
        "#);

        assert_eq!(actual.out, "123");
    }

    #[test]
    fn properly_captures_empty_column() {
        let actual = nu!(r#"
            echo ["1:INFO:component:all is well" "2:ERROR::something bad happened"]
            | parse "{timestamp}:{level}:{tag}:{entry}"
            | get entry
            | get 1
        "#);

        assert_eq!(actual.out, "something bad happened");
    }

    #[test]
    fn errors_when_missing_closing_brace() {
        let actual = nu!(r#"
            echo "(abc)123"
            | parse "(abc){name"
            | get name
        "#);

        assert!(
            actual
                .err
                .contains("Found opening `{` without an associated closing `}`")
        );
    }

    #[test]
    fn ignore_multiple_placeholder() {
        let actual = nu!(r#"
            echo ["1:INFO:component:all is well" "2:ERROR::something bad happened"]
            | parse "{_}:{level}:{_}:{entry}"
            | to json -r
        "#);

        assert_eq!(
            actual.out,
            r#"[{"level":"INFO","entry":"all is well"},{"level":"ERROR","entry":"something bad happened"}]"#
        );
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
            sandbox.with_files(&nushell_git_log_oneline());

            let actual = nu!(cwd: dirs.test(), r#"
                open nushell_git_log_oneline.txt
                | parse --regex "(?P<Hash>\\w+) (?P<Message>.+) \\(#(?P<PR>\\d+)\\)"
                | get 1
                | get PR
            "#);

            assert_eq!(actual.out, "1842");
        })
    }

    #[test]
    fn extracts_fields_with_all_unnamed_groups() {
        Playground::setup("parse_test_regex_2", |dirs, sandbox| {
            sandbox.with_files(&nushell_git_log_oneline());

            let actual = nu!(cwd: dirs.test(), r#"
                open nushell_git_log_oneline.txt
                | parse --regex "(\\w+) (.+) \\(#(\\d+)\\)"
                | get 1
                | get capture0
            "#);

            assert_eq!(actual.out, "b89976da");
        })
    }

    #[test]
    fn extracts_fields_with_named_and_unnamed_groups() {
        Playground::setup("parse_test_regex_3", |dirs, sandbox| {
            sandbox.with_files(&nushell_git_log_oneline());

            let actual = nu!(cwd: dirs.test(), r#"
                open nushell_git_log_oneline.txt
                | parse --regex "(?P<Hash>\\w+) (.+) \\(#(?P<PR>\\d+)\\)"
                | get 1
                | get capture1
            "#);

            assert_eq!(actual.out, "let format access variables also");
        })
    }

    #[test]
    fn errors_with_invalid_regex() {
        Playground::setup("parse_test_regex_1", |dirs, sandbox| {
            sandbox.with_files(&nushell_git_log_oneline());

            let actual = nu!(cwd: dirs.test(), r#"
                open nushell_git_log_oneline.txt
                | parse --regex "(?P<Hash>\\w+ unfinished capture group"
            "#);

            assert!(
                actual
                    .err
                    .contains("Opening parenthesis without closing parenthesis")
            );
        })
    }

    #[test]
    fn parse_works_with_streaming() {
        let actual =
            nu!(r#"seq char a z | each {|c| $c + " a"} | parse '{letter} {a}' | describe"#);

        assert_eq!(actual.out, "table<letter: string, a: string> (stream)")
    }

    #[test]
    fn parse_does_not_truncate_list_streams() {
        let actual = nu!(r#"
            [a b c]
            | each {|x| $x}
            | parse --regex "[ac]"
            | length
        "#);

        assert_eq!(actual.out, "2");
    }

    #[test]
    fn parse_handles_external_stream_chunking() {
        Playground::setup("parse_test_streaming_1", |dirs, sandbox| {
            let data: String = "abcdefghijklmnopqrstuvwxyz".repeat(1000);
            sandbox.with_files(&[Stub::FileWithContent("data.txt", &data)]);

            let actual = nu!(
                cwd: dirs.test(),
                r#"open data.txt | parse --regex "(abcdefghijklmnopqrstuvwxyz)" | length"#
            );

            assert_eq!(actual.out, "1000");
        })
    }
}
