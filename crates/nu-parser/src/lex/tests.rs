use nu_source::{Span, SpannedItem};

use super::lexer::*;
use super::tokens::*;

fn span(left: usize, right: usize) -> Span {
    Span::new(left, right)
}

mod bare {

    use super::*;

    #[test]
    fn simple_1() {
        let input = "foo bar baz";

        let (result, err) = lex(input, 0);

        assert!(err.is_none());
        assert_eq!(result[0].span, span(0, 3));
    }

    #[test]
    fn simple_2() {
        let input = "'foo bar' baz";

        let (result, err) = lex(input, 0);

        assert!(err.is_none());
        assert_eq!(result[0].span, span(0, 9));
    }

    #[test]
    fn simple_3() {
        let input = "'foo\" bar' baz";

        let (result, err) = lex(input, 0);

        assert!(err.is_none());
        assert_eq!(result[0].span, span(0, 10));
    }

    #[test]
    fn simple_4() {
        let input = "[foo bar] baz";

        let (result, err) = lex(input, 0);

        assert!(err.is_none());
        assert_eq!(result[0].span, span(0, 9));
    }

    #[test]
    fn simple_5() {
        let input = "'foo 'bar baz";

        let (result, err) = lex(input, 0);

        assert!(err.is_none());
        assert_eq!(result[0].span, span(0, 9));
    }

    #[test]
    fn simple_6() {
        let input = "''foo baz";

        let (result, err) = lex(input, 0);

        assert!(err.is_none());
        assert_eq!(result[0].span, span(0, 5));
    }

    #[test]
    fn simple_7() {
        let input = "'' foo";

        let (result, err) = lex(input, 0);

        assert!(err.is_none());
        assert_eq!(result[0].span, span(0, 2));
    }

    #[test]
    fn simple_8() {
        let input = " '' foo";

        let (result, err) = lex(input, 0);

        assert!(err.is_none());
        assert_eq!(result[0].span, span(1, 3));
    }

    #[test]
    fn simple_9() {
        let input = " 'foo' foo";

        let (result, err) = lex(input, 0);

        assert!(err.is_none());
        assert_eq!(result[0].span, span(1, 6));
    }

    #[test]
    fn simple_10() {
        let input = "[foo, bar]";

        let (result, err) = lex(input, 0);

        assert!(err.is_none());
        assert_eq!(result[0].span, span(0, 10));
    }

    #[test]
    fn lex_comment() {
        let input = r#"
#A comment
def e [] {echo hi}
            "#;

        let (result, err) = lex(input, 0);
        assert!(err.is_none());

        //result[0] == EOL
        assert_eq!(result[1].span, span(2, 11));
        assert_eq!(
            result[1].contents,
            TokenContents::Comment(LiteComment::new(
                "A comment".to_string().spanned(Span::new(2, 11))
            ))
        );
    }

    #[test]
    fn def_comment_with_sinqle_quote() {
        let input = r#"def f [] {
	    	# shouldn't return error
			echo hi
		}"#;
        let (_result, err) = lex(input, 0);
        assert!(err.is_none());
    }

    #[test]
    fn def_comment_with_double_quote() {
        let input = r#"def f [] {
	    	# should "not return error
			echo hi
		}"#;
        let (_result, err) = lex(input, 0);
        assert!(err.is_none());
    }

    #[test]
    fn def_comment_with_bracks() {
        let input = r#"def f [] {
	    	# should not [return error
			echo hi
		}"#;
        let (_result, err) = lex(input, 0);
        assert!(err.is_none());
    }

    #[test]
    fn def_comment_with_curly() {
        let input = r#"def f [] {
	    	# should not return {error
			echo hi
		}"#;
        let (_result, err) = lex(input, 0);
        assert!(err.is_none());
    }

    #[test]
    fn ignore_future() {
        let input = "foo 'bar";

        let (result, _) = lex(input, 0);

        assert_eq!(result[0].span, span(0, 3));
    }

    #[test]
    fn invalid_1() {
        let input = "'foo bar";

        let (_, err) = lex(input, 0);

        assert!(err.is_some());
    }

    #[test]
    fn invalid_2() {
        let input = "'bar";

        let (_, err) = lex(input, 0);

        assert!(err.is_some());
    }

    #[test]
    fn invalid_4() {
        let input = " 'bar";

        let (_, err) = lex(input, 0);

        assert!(err.is_some());
    }
}

mod lite_parse {
    use nu_source::HasSpan;

    use super::*;

    #[test]
    fn pipeline() {
        let (result, err) = lex("cmd1 | cmd2 ; deploy", 0);
        assert!(err.is_none());
        let (result, err) = parse_block(result);
        assert!(err.is_none());
        assert_eq!(result.span(), span(0, 20));
        assert_eq!(result.block[0].pipelines[0].span(), span(0, 11));
        assert_eq!(result.block[0].pipelines[1].span(), span(14, 20));
    }

    #[test]
    fn simple_1() {
        let (result, err) = lex("foo", 0);
        assert!(err.is_none());
        let (result, err) = parse_block(result);
        assert!(err.is_none());
        assert_eq!(result.block.len(), 1);
        assert_eq!(result.block[0].pipelines.len(), 1);
        assert_eq!(result.block[0].pipelines[0].commands.len(), 1);
        assert_eq!(result.block[0].pipelines[0].commands[0].parts.len(), 1);
        assert_eq!(
            result.block[0].pipelines[0].commands[0].parts[0].span,
            span(0, 3)
        );
    }

    #[test]
    fn simple_offset() {
        let (result, err) = lex("foo", 10);
        assert!(err.is_none());
        let (result, err) = parse_block(result);
        assert!(err.is_none());
        assert_eq!(result.block[0].pipelines.len(), 1);
        assert_eq!(result.block[0].pipelines[0].commands.len(), 1);
        assert_eq!(result.block[0].pipelines[0].commands[0].parts.len(), 1);
        assert_eq!(
            result.block[0].pipelines[0].commands[0].parts[0].span,
            span(10, 13)
        );
    }

    #[test]
    fn incomplete_result() {
        let (result, err) = lex("my_command \"foo' --test", 10);
        assert!(matches!(
            err.unwrap().reason(),
            nu_errors::ParseErrorReason::Eof { .. }
        ));
        let (result, _) = parse_block(result);

        assert_eq!(result.block.len(), 1);
        assert_eq!(result.block[0].pipelines.len(), 1);
        assert_eq!(result.block[0].pipelines[0].commands.len(), 1);
        assert_eq!(result.block[0].pipelines[0].commands[0].parts.len(), 2);

        assert_eq!(
            result.block[0].pipelines[0].commands[0].parts[0].item,
            "my_command"
        );
        assert_eq!(
            result.block[0].pipelines[0].commands[0].parts[1].item,
            "\"foo' --test\""
        );
    }
    #[test]
    fn command_with_comment() {
        let code = r#"
# My echo
# * It's much better :)
def my_echo [arg] { echo $arg }
        "#;
        let (result, err) = lex(code, 0);
        assert!(err.is_none());
        let (result, err) = parse_block(result);
        assert!(err.is_none());

        assert_eq!(result.block.len(), 1);
        assert_eq!(result.block[0].pipelines.len(), 1);
        assert_eq!(result.block[0].pipelines[0].commands.len(), 1);
        assert_eq!(result.block[0].pipelines[0].commands[0].parts.len(), 4);
        assert_eq!(
            result.block[0].pipelines[0].commands[0].comments,
            Some(vec![
                //Leading space is trimmed
                LiteComment::new_with_ws(
                    " ".to_string().spanned(Span::new(2, 3)),
                    "My echo".to_string().spanned(Span::new(3, 10))
                ),
                LiteComment::new_with_ws(
                    " ".to_string().spanned(Span::new(12, 13)),
                    "* It's much better :)"
                        .to_string()
                        .spanned(Span::new(13, 34))
                )
            ])
        );
    }
    #[test]
    fn discarded_comment() {
        let code = r#"
# This comment gets discarded, because of the following empty line

echo 42
        "#;
        let (result, err) = lex(code, 0);
        assert!(err.is_none());
        // assert_eq!(format!("{:?}", result), "");
        let (result, err) = parse_block(result);
        assert!(err.is_none());
        assert_eq!(result.block.len(), 1);
        assert_eq!(result.block[0].pipelines.len(), 1);
        assert_eq!(result.block[0].pipelines[0].commands.len(), 1);
        assert_eq!(result.block[0].pipelines[0].commands[0].parts.len(), 2);
        assert_eq!(result.block[0].pipelines[0].commands[0].comments, None);
    }
}

#[test]
fn no_discarded_white_space_start_of_comment() {
    let code = r#"
#No white_space at firt line ==> No white_space discarded
#   Starting space is not discarded
echo 42
        "#;
    let (result, err) = lex(code, 0);
    assert!(err.is_none());
    // assert_eq!(format!("{:?}", result), "");
    let (result, err) = parse_block(result);
    assert!(err.is_none());
    assert_eq!(result.block.len(), 1);
    assert_eq!(result.block[0].pipelines.len(), 1);
    assert_eq!(result.block[0].pipelines[0].commands.len(), 1);
    assert_eq!(result.block[0].pipelines[0].commands[0].parts.len(), 2);
    assert_eq!(
        result.block[0].pipelines[0].commands[0].comments,
        Some(vec![
            LiteComment::new(
                "No white_space at firt line ==> No white_space discarded"
                    .to_string()
                    .spanned(Span::new(2, 58))
            ),
            LiteComment::new(
                "   Starting space is not discarded"
                    .to_string()
                    .spanned(Span::new(60, 94))
            ),
        ])
    );
}

#[test]
fn multiple_discarded_white_space_start_of_comment() {
    let code = r#"
#  Discard 2 spaces
# Discard 1 space
#  Discard 2 spaces
echo 42
        "#;
    let (result, err) = lex(code, 0);
    assert!(err.is_none());
    // assert_eq!(format!("{:?}", result), "");
    let (result, err) = parse_block(result);
    assert!(err.is_none());
    assert_eq!(result.block.len(), 1);
    assert_eq!(result.block[0].pipelines.len(), 1);
    assert_eq!(result.block[0].pipelines[0].commands.len(), 1);
    assert_eq!(result.block[0].pipelines[0].commands[0].parts.len(), 2);
    assert_eq!(
        result.block[0].pipelines[0].commands[0].comments,
        Some(vec![
            LiteComment::new_with_ws(
                "  ".to_string().spanned(Span::new(2, 4)),
                "Discard 2 spaces".to_string().spanned(Span::new(4, 20))
            ),
            LiteComment::new_with_ws(
                " ".to_string().spanned(Span::new(22, 23)),
                "Discard 1 space".to_string().spanned(Span::new(23, 38))
            ),
            LiteComment::new_with_ws(
                "  ".to_string().spanned(Span::new(40, 42)),
                "Discard 2 spaces".to_string().spanned(Span::new(42, 58))
            ),
        ])
    );
}
