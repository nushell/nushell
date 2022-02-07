use nu_parser::{lex, ParseError, Token, TokenContents};
use nu_protocol::Span;

#[test]
fn lex_basic() {
    let file = b"let x = 4";

    let output = lex(file, 0, &[], &[], true);

    assert!(output.1.is_none());
}

#[test]
fn lex_newline() {
    let file = b"let x = 300\nlet y = 500;";

    let output = lex(file, 0, &[], &[], true);

    assert!(output.0.contains(&Token {
        contents: TokenContents::Eol,
        span: Span { start: 11, end: 12 }
    }));
}

#[test]
fn lex_empty() {
    let file = b"";

    let output = lex(file, 0, &[], &[], true);

    assert!(output.0.is_empty());
    assert!(output.1.is_none());
}

#[test]
fn lex_parenthesis() {
    // The whole parenthesis is an item for the lexer
    let file = b"let x = (300 + (322 * 444));";

    let output = lex(file, 0, &[], &[], true);

    assert_eq!(
        output.0.get(3).unwrap(),
        &Token {
            contents: TokenContents::Item,
            span: Span { start: 8, end: 27 }
        }
    );
}

#[test]
fn lex_comment() {
    let file = b"let x = 300 # a comment \n $x + 444";

    let output = lex(file, 0, &[], &[], false);

    assert_eq!(
        output.0.get(4).unwrap(),
        &Token {
            contents: TokenContents::Comment,
            span: Span { start: 12, end: 24 }
        }
    );
}

#[test]
fn lex_is_incomplete() {
    let file = b"let x = 300 | ;";

    let output = lex(file, 0, &[], &[], true);

    let err = output.1.unwrap();
    assert!(matches!(err, ParseError::ExtraTokens(_)));
}

#[test]
fn lex_incomplete_paren() {
    let file = b"let x = (300 + ( 4 + 1)";

    let output = lex(file, 0, &[], &[], true);

    let err = output.1.unwrap();
    assert!(matches!(err, ParseError::UnexpectedEof(v, _) if v == ")"));
}

#[test]
fn lex_incomplete_quote() {
    let file = b"let x = '300 + 4 + 1";

    let output = lex(file, 0, &[], &[], true);

    let err = output.1.unwrap();
    assert!(matches!(err, ParseError::UnexpectedEof(v, _) if v == "'"));
}

#[test]
fn lex_comments() {
    // Comments should keep the end of line token
    // Code:
    // let z = 4
    // let x = 4 #comment
    // let y = 1 # comment
    let file = b"let z = 4 #comment \n let x = 4 # comment\n let y = 1 # comment";

    let output = lex(file, 0, &[], &[], false);

    assert_eq!(
        output.0.get(4).unwrap(),
        &Token {
            contents: TokenContents::Comment,
            span: Span { start: 10, end: 19 }
        }
    );
    assert_eq!(
        output.0.get(5).unwrap(),
        &Token {
            contents: TokenContents::Eol,
            span: Span { start: 19, end: 20 }
        }
    );

    // When there is no space between the comment and the new line the span
    // for the command and the EOL overlaps
    assert_eq!(
        output.0.get(10).unwrap(),
        &Token {
            contents: TokenContents::Comment,
            span: Span { start: 31, end: 40 }
        }
    );
    assert_eq!(
        output.0.get(11).unwrap(),
        &Token {
            contents: TokenContents::Eol,
            span: Span { start: 40, end: 41 }
        }
    );
}
