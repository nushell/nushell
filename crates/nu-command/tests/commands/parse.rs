use indoc::indoc;
use nu_protocol::{ByteStream, ByteStreamType, PipelineData, ShellError, Signals, Span};
use nu_test_support::{fs::Stub, prelude::*};
use pretty_assertions::assert_matches;
use rstest::rstest;

mod simple {
    use super::*;

    #[test]
    fn extracts_fields_from_the_given_the_pattern() -> Result {
        let input = indoc! {"
            VAR1=Cheese
            VAR2=JTParsed
            VAR3=NushellSecretIngredient
        "};

        let code = r#"
            $in
            | lines
            | each {|it| $it | parse "{Name}={Value}" }
            | flatten
            | get 1.Value
        "#;

        test()
            .run_with_data(code, input)
            .expect_value_eq("JTParsed")
    }

    #[test]
    fn double_open_curly_evaluates_to_a_single_curly() -> Result {
        let code = r#"
            "{abc}123"
            | parse "{{abc}{name}"
            | get name.0
        "#;

        test().run(code).expect_value_eq("123")
    }

    #[test]
    fn char_lbrace_before_capture() -> Result {
        test()
            .run(r#""1234{56" | parse $'{a}(char lbrace){b}' | get a.0"#)
            .expect_value_eq("1234")
    }

    #[test]
    fn double_brace_at_end_matches_literal_brace_with_capture() -> Result {
        test()
            .run(r#""{hello" | parse "{{foo}" | get foo.0"#)
            .expect_value_eq("hello")
    }

    #[test]
    fn double_brace_at_end_does_not_match_without_brace_in_input() -> Result {
        test()
            .run(r#""hello" | parse "{{foo}" | length"#)
            .expect_value_eq(0)
    }

    #[test]
    fn double_brace_with_suffix_before_capture_stays_literal() -> Result {
        test()
            .run(r#""{foo}x123" | parse "{{foo}x{bar}" | get bar.0"#)
            .expect_value_eq("123")
    }

    #[test]
    fn properly_escapes_text() -> Result {
        let code = r#"
            "(abc)123"
            | parse "(abc){name}"
            | get name.0
        "#;

        test().run(code).expect_value_eq("123")
    }

    #[test]
    fn properly_captures_empty_column() -> Result {
        let code = r#"
            ["1:INFO:component:all is well" "2:ERROR::something bad happened"]
            | parse "{timestamp}:{level}:{tag}:{entry}"
            | get entry.1
        "#;

        test().run(code).expect_value_eq("something bad happened")
    }

    #[rstest]
    #[case::unclosed_name("(abc){name")]
    #[case::lone_open("{")]
    #[case::trailing_open("hello {")]
    #[case::closed_then_open("{name}{")]
    fn errors_when_missing_closing_brace(#[case] pattern: &str) -> Result {
        let err = test()
            .run_with_data(r#"let pattern = $in; "(abc)123" | parse $pattern"#, pattern)
            .expect_shell_error()?;
        assert_matches!(
            err,
            ShellError::DelimiterError { msg, .. }
                if msg == "Found opening `{` without an associated closing `}`"
        );

        Ok(())
    }

    #[test]
    fn escaped_open_brace_is_literal() -> Result {
        test()
            .run(r#""{" | parse "{{" | length"#)
            .expect_value_eq(1)
    }

    #[test]
    fn unmatched_close_brace_is_literal() -> Result {
        test()
            .run(r#""hello}" | parse "hello}" | length"#)
            .expect_value_eq(1)
    }

    #[test]
    fn ignore_multiple_placeholder() -> Result {
        let code = r#"
            ["1:INFO:component:all is well" "2:ERROR::something bad happened"]
            | parse "{_}:{level}:{_}:{entry}"
        "#;

        test().run(code).expect_value_eq(test_table![
            ["level", "entry"];
            ["INFO", "all is well"],
            ["ERROR", "something bad happened"],
        ])
    }
}

mod regex {
    use super::*;

    fn nushell_git_log_oneline() -> &'static str {
        "ae87582c Fix missing invocation errors (#1846)\nb89976da let format access variables also (#1842)"
    }

    #[test]
    fn extracts_fields_with_all_named_groups() -> Result {
        let code = r#"
            $in
            | parse --regex "(?P<Hash>\\w+) (?P<Message>.+) \\(#(?P<PR>\\d+)\\)"
            | get 1.PR
        "#;

        test()
            .run_with_data(code, nushell_git_log_oneline())
            .expect_value_eq("1842")
    }

    #[test]
    fn extracts_fields_with_all_unnamed_groups() -> Result {
        let code = r#"
            $in
            | parse --regex "(\\w+) (.+) \\(#(\\d+)\\)"
            | get 1.capture0
        "#;

        test()
            .run_with_data(code, nushell_git_log_oneline())
            .expect_value_eq("b89976da")
    }

    #[test]
    fn extracts_fields_with_named_and_unnamed_groups() -> Result {
        let code = r#"
            $in
            | parse --regex "(?P<Hash>\\w+) (.+) \\(#(?P<PR>\\d+)\\)"
            | get 1.capture1
        "#;

        test()
            .run_with_data(code, nushell_git_log_oneline())
            .expect_value_eq("let format access variables also")
    }

    #[test]
    fn errors_with_invalid_regex() -> Result {
        let code = r#"
            $in
            | parse --regex "(?P<Hash>\\w+ unfinished capture group"
        "#;

        let err = test()
            .run_with_data(code, nushell_git_log_oneline())
            .expect_shell_error()?;
        let ShellError::InvalidValue { actual, .. } = err else {
            panic!("expected InvalidValue, got {err:?}");
        };
        assert_contains("Opening parenthesis without closing parenthesis", actual);

        Ok(())
    }

    #[test]
    fn parse_works_with_streaming() -> Result {
        test()
            .run(r#"seq char a z | each {|c| $c + " a"} | parse '{letter} {a}' | describe"#)
            .expect_value_eq("table<letter: string, a: string> (stream)")
    }

    #[test]
    fn parse_does_not_truncate_list_streams() -> Result {
        let code = r#"
            [a b c]
            | each {|x| $x}
            | parse --regex "[ac]"
            | length
        "#;

        test().run(code).expect_value_eq(2)
    }

    #[test]
    fn parse_handles_external_stream_chunking() -> Result {
        Playground::setup("parse_test_streaming_1", |dirs, sandbox| {
            let data: String = "abcdefghijklmnopqrstuvwxyz".repeat(1000);
            sandbox.with_files(&[Stub::FileWithContent("data.txt", &data)]);

            test()
                .cwd(dirs.test())
                .run(r#"open data.txt | parse --regex "(abcdefghijklmnopqrstuvwxyz)" | length"#)
                .expect_value_eq(1000)
        })
    }

    #[test]
    fn multiline_regex() -> Result {
        let mut tester = test();

        let pattern = r#"(?ms)^(?<n>\d+)\. (?<text>.*?)(?=$\s^\d|\Z)"#;
        let () = tester.run_with_data("let pattern = $in", pattern)?;

        let input = [
            "1. one\n",
            "2. two and\n",
            "   a half\n",
            "3. three\n",
            "4. four and\n",
            "   some more\n",
        ];
        let byte_stream = PipelineData::ByteStream(
            ByteStream::from_iter(
                input,
                Span::test_data(),
                Signals::empty(),
                ByteStreamType::Unknown,
            ),
            None,
        );

        let code = "parse -r $pattern";
        tester
            .run_raw_with_data(code, byte_stream)
            .and_then(|x| x.body.into_value(Span::test_data()).map_err(Error::from))
            .expect_value_eq(test_table![
              ["n", "text"];
              ["1", "one"],
              ["2", "two and\n   a half"],
              ["3", "three"],
              ["4", "four and\n   some more"]
            ])
    }
}
