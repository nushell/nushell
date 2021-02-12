///All of the functions in this mod parse only 1 Token per invocation.
///Therefore they are primitives
use crate::lex::{lexer::Token, tokens::TokenContents};
use crate::parse::util::token_to_spanned_string;
use nu_errors::ParseError;
use nu_protocol::SyntaxShape;
use nu_source::{Span, Spanned, SpannedItem};

///Helper function
pub(crate) fn is_baseline_token_matching(token: &Token, string: &str) -> bool {
    match &token.contents {
        TokenContents::Baseline(base) => base == string,
        _ => false,
    }
}

pub(crate) fn parse_comma(tokens: &[Token]) -> (bool, usize) {
    fn is_comma(token: &Token) -> bool {
        is_baseline_token_matching(token, ",")
    }
    if !tokens.is_empty() && is_comma(&tokens[0]) {
        (true, 1)
    } else {
        (false, 0)
    }
}

pub(crate) fn parse_eol(tokens: &[Token]) -> (bool, usize) {
    if !tokens.is_empty() && tokens[0].contents.is_eol() {
        (true, 1)
    } else {
        (false, 0)
    }
}

pub(crate) fn parse_optional_comment(tokens: &[Token]) -> (Option<String>, usize) {
    let mut comment_text = None;
    let mut i: usize = 0;
    if i < tokens.len() {
        if let TokenContents::Comment(comment) = &tokens[i].contents {
            comment_text = Some(comment.trim().to_string());
            i += 1;
        }
    }
    (comment_text, i)
}

///Returns true if token is optional modifier
pub(crate) fn parse_optional_parameter_optional_modifier(token: &Token) -> (bool, usize) {
    if is_baseline_token_matching(token, "?") {
        (true, 1)
    } else {
        (false, 0)
    }
}

pub(crate) fn parse_flag_optional_shortform(
    tokens: &[Token],
) -> (Option<char>, usize, Option<ParseError>) {
    if tokens.is_empty() {
        return (None, 0, None);
    }

    let token = &tokens[0];
    return if let TokenContents::Baseline(shortform) = &token.contents {
        let mut chars = shortform.chars();
        match (chars.next(), chars.next_back()) {
            (Some('('), Some(')')) => {
                let mut err = None;

                let flag_span = Span::new(
                    token.span.start() + 1, //Skip '('
                    token.span.end() - 1,   // Skip ')'
                );

                let c: String = chars.collect();
                let dash_count = c.chars().take_while(|c| *c == '-').count();
                err = err
                    .or_else(|| err_on_too_many_dashes(dash_count, c.clone().spanned(flag_span)));
                let name = &c[dash_count..];
                err = err.or_else(|| err_on_name_too_long(name, c.clone().spanned(flag_span)));
                let c = name.chars().next();

                (c, 1, err)
            }
            _ => (None, 0, None),
        }
    } else {
        (None, 0, None)
    };

    fn err_on_too_many_dashes(dash_count: usize, actual: Spanned<String>) -> Option<ParseError> {
        match dash_count {
            0 => {
                //If no starting -
                Some(ParseError::mismatch("Shortflag starting with '-'", actual))
            }
            1 => None,
            _ => {
                //If --
                Some(ParseError::mismatch(
                    "Shortflag starting with a single '-'",
                    actual,
                ))
            }
        }
    }

    fn err_on_name_too_long(name: &str, actual: Spanned<String>) -> Option<ParseError> {
        if name.len() != 1 {
            Some(ParseError::mismatch(
                "Shortflag of exactly 1 character",
                actual,
            ))
        } else {
            None
        }
    }
}

pub(crate) fn parse_flag_name(token: &Token) -> (Spanned<String>, Option<ParseError>) {
    if let TokenContents::Baseline(name) = &token.contents {
        if !name.starts_with("--") {
            (
                name.clone().spanned(token.span),
                Some(ParseError::mismatch(
                    "longform of a flag (Starting with --)",
                    token_to_spanned_string(token),
                )),
            )
        } else {
            //Discard preceding --
            let name = name[2..].to_string();
            (name.spanned(token.span), None)
        }
    } else {
        (
            "".to_string().spanned_unknown(),
            Some(ParseError::mismatch(
                "longform of a flag (Starting with --)",
                token_to_spanned_string(token),
            )),
        )
    }
}

pub(crate) fn parse_param_name(token: &Token) -> (Spanned<String>, Option<ParseError>) {
    if let TokenContents::Baseline(name) = &token.contents {
        let name = name.clone().spanned(token.span);
        (name, None)
    } else {
        (
            "InternalError".to_string().spanned(token.span),
            Some(ParseError::mismatch(
                "parameter name",
                token_to_spanned_string(token),
            )),
        )
    }
}

pub fn parse_type_token(type_: &Token) -> (SyntaxShape, Option<ParseError>) {
    match &type_.contents {
        TokenContents::Baseline(type_str) => match type_str.as_str() {
            "int" => (SyntaxShape::Int, None),
            "string" => (SyntaxShape::String, None),
            "path" => (SyntaxShape::FilePath, None),
            "table" => (SyntaxShape::Table, None),
            "unit" => (SyntaxShape::Unit, None),
            "number" => (SyntaxShape::Number, None),
            "pattern" => (SyntaxShape::GlobPattern, None),
            "range" => (SyntaxShape::Range, None),
            "block" => (SyntaxShape::Block, None),
            "any" => (SyntaxShape::Any, None),
            _ => (
                SyntaxShape::Any,
                Some(ParseError::mismatch("type", token_to_spanned_string(type_))),
            ),
        },
        _ => (
            SyntaxShape::Any,
            Some(ParseError::mismatch("type", token_to_spanned_string(type_))),
        ),
    }
}

pub(crate) fn parse_rest_name(name_token: &Token) -> Option<ParseError> {
    return if let TokenContents::Baseline(name) = &name_token.contents {
        if !name.starts_with("...") {
            Some(parse_rest_name_err(name_token))
        } else if !name.starts_with("...rest") {
            Some(ParseError::mismatch(
                "rest argument name to be 'rest'",
                token_to_spanned_string(name_token),
            ))
        } else {
            None
        }
    } else {
        Some(parse_rest_name_err(name_token))
    };

    fn parse_rest_name_err(token: &Token) -> ParseError {
        ParseError::mismatch("...rest", token_to_spanned_string(token))
    }
}
