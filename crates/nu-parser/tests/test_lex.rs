use nu_parser::{lex, lex_signature, Token, TokenContents};
use nu_protocol::{ActualSpan, ParseError};

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
        span: ActualSpan::new(11, 12)
    }));
}

#[test]
fn lex_annotations_list() {
    let file = b"items: list<string>";

    let (output, err) = lex_signature(file, 0, &[b'\n', b'\r'], &[b':', b'=', b','], false);

    assert!(err.is_none());
    assert_eq!(output.len(), 3);
}

#[test]
fn lex_annotations_record() {
    let file = b"config: record<name: string>";

    let (output, err) = lex_signature(file, 0, &[b'\n', b'\r'], &[b':', b'=', b','], false);

    assert!(err.is_none());
    assert_eq!(output.len(), 3);
}

#[test]
fn lex_annotations_empty() {
    let file = b"items: list<>";

    let (output, err) = lex_signature(file, 0, &[b'\n', b'\r'], &[b':', b'=', b','], false);

    assert!(err.is_none());
    assert_eq!(output.len(), 3);
}

#[test]
fn lex_annotations_space_before_annotations() {
    let file = b"items: list <string>";

    let (output, err) = lex_signature(file, 0, &[b'\n', b'\r'], &[b':', b'=', b','], false);

    assert!(err.is_none());
    assert_eq!(output.len(), 4);
}

#[test]
fn lex_annotations_space_within_annotations() {
    let file = b"items: list< string>";

    let (output, err) = lex_signature(file, 0, &[b'\n', b'\r'], &[b':', b'=', b','], false);

    assert!(err.is_none());
    assert_eq!(output.len(), 3);

    let file = b"items: list<string >";

    let (output, err) = lex_signature(file, 0, &[b'\n', b'\r'], &[b':', b'=', b','], false);

    assert!(err.is_none());
    assert_eq!(output.len(), 3);

    let file = b"items: list< string >";

    let (output, err) = lex_signature(file, 0, &[b'\n', b'\r'], &[b':', b'=', b','], false);

    assert!(err.is_none());
    assert_eq!(output.len(), 3);
}

#[test]
fn lex_annotations_nested() {
    let file = b"items: list<record<name: string>>";

    let (output, err) = lex_signature(file, 0, &[b'\n', b'\r'], &[b':', b'=', b','], false);

    assert!(err.is_none());
    assert_eq!(output.len(), 3);
}

#[test]
fn lex_annotations_nested_unterminated() {
    let file = b"items: list<record<name: string>";

    let (output, err) = lex_signature(file, 0, &[b'\n', b'\r'], &[b':', b'=', b','], false);

    assert!(matches!(err.unwrap(), ParseError::UnexpectedEof(_, _)));
    assert_eq!(output.len(), 3);
}

#[test]
fn lex_annotations_unterminated() {
    let file = b"items: list<string";

    let (output, err) = lex_signature(file, 0, &[b'\n', b'\r'], &[b':', b'=', b','], false);

    assert!(matches!(err.unwrap(), ParseError::UnexpectedEof(_, _)));
    assert_eq!(output.len(), 3);
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
            span: ActualSpan::new(8, 27)
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
            span: ActualSpan::new(12, 24)
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
fn lex_comments_no_space() {
    // test for parses that contain tokens that normally introduce comments
    // Code:
    // let z = 42 #the comment
    // let x#y = 69 #hello
    // let flk = nixpkgs#hello #hello
    let file = b"let z = 42 #the comment \n let x#y = 69 #hello \n let flk = nixpkgs#hello #hello";
    let output = lex(file, 0, &[], &[], false);

    assert_eq!(
        output.0.get(4).unwrap(),
        &Token {
            contents: TokenContents::Comment,
            span: ActualSpan::new(11, 24)
        }
    );

    assert_eq!(
        output.0.get(7).unwrap(),
        &Token {
            contents: TokenContents::Item,
            span: ActualSpan::new(30, 33)
        }
    );

    assert_eq!(
        output.0.get(10).unwrap(),
        &Token {
            contents: TokenContents::Comment,
            span: ActualSpan::new(39, 46)
        }
    );

    assert_eq!(
        output.0.get(15).unwrap(),
        &Token {
            contents: TokenContents::Item,
            span: ActualSpan::new(58, 71)
        }
    );

    assert_eq!(
        output.0.get(16).unwrap(),
        &Token {
            contents: TokenContents::Comment,
            span: ActualSpan::new(72, 78)
        }
    );
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
            span: ActualSpan::new(10, 19)
        }
    );
    assert_eq!(
        output.0.get(5).unwrap(),
        &Token {
            contents: TokenContents::Eol,
            span: ActualSpan::new(19, 20)
        }
    );

    // When there is no space between the comment and the new line the span
    // for the command and the EOL overlaps
    assert_eq!(
        output.0.get(10).unwrap(),
        &Token {
            contents: TokenContents::Comment,
            span: ActualSpan::new(31, 40)
        }
    );
    assert_eq!(
        output.0.get(11).unwrap(),
        &Token {
            contents: TokenContents::Eol,
            span: ActualSpan::new(40, 41)
        }
    );
}
