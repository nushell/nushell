use nu_parser::{lex, ParseError, Span, Token, TokenContents};

#[test]
fn lex_basic() {
    let file = b"let x = 4";

    let output = lex(file, 0, &[], &[]);

    assert!(output.1.is_none());
}

#[test]
fn lex_newline() {
    let file = b"let x = 300\nlet y = 500;";

    let output = lex(file, 0, &[], &[]);

    assert!(output.0.contains(&Token {
        contents: TokenContents::Eol,
        span: Span { start: 11, end: 12 }
    }));
}

#[test]
fn lex_empty() {
    let file = b"";

    let output = lex(file, 0, &[], &[]);

    assert!(output.0.is_empty());
    assert!(output.1.is_none());
}

#[test]
fn lex_parenthesis() {
    // The whole parenthesis is an item for the lexer
    let file = b"let x = (300 + (322 * 444));";

    let output = lex(file, 0, &[], &[]);

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

    let output = lex(file, 0, &[], &[]);

    assert_eq!(
        output.0.get(4).unwrap(),
        &Token {
            contents: TokenContents::Comment,
            span: Span { start: 12, end: 25 }
        }
    );
}

#[test]
fn lex_is_incomplete() {
    let file = b"let x = 300 | ;";

    let output = lex(file, 0, &[], &[]);

    let err = output.1.unwrap();
    assert!(matches!(err, ParseError::ExtraTokens(_)));
}

#[test]
fn lex_incomplete_paren() {
    let file = b"let x = (300 + ( 4 + 1)";

    let output = lex(file, 0, &[], &[]);

    let err = output.1.unwrap();
    assert!(matches!(err, ParseError::UnexpectedEof(v, _) if v == ")"));
}

#[test]
fn lex_incomplete_quote() {
    let file = b"let x = '300 + 4 + 1";

    let output = lex(file, 0, &[], &[]);

    let err = output.1.unwrap();
    assert!(matches!(err, ParseError::UnexpectedEof(v, _) if v == "'"));
}
