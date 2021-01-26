use crate::{
    lex::{Token, TokenContents},
    parse::def::parse_lib::Parse,
    parse::util::token_to_spanned_string,
};
use log::debug;
use nu_errors::ParseError;
use nu_protocol::SyntaxShape;
use nu_source::{Span, Spanned, SpannedItem};

fn parse_type(type_: &Spanned<String>) -> (SyntaxShape, Option<ParseError>) {
    debug!("Parsing type {:?}", type_);
    match type_.item.as_str() {
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
            Some(ParseError::mismatch("type", type_.clone())),
        ),
    }
}

///Better use Type!
///Type parses (: shape)?
pub(crate) struct Shape;
impl Parse for Shape {
    type Output = SyntaxShape;

    fn parse(tokens: &[Token], i: usize) -> (Self::Output, usize, Option<ParseError>) {
        let shape_token = &tokens[i];
        match &shape_token.contents {
            TokenContents::Baseline(type_str) => {
                let (shape, err) = parse_type(&type_str.clone().spanned(shape_token.span));
                (shape, i + 1, err)
            }
            _ => Self::mismatch_default_return(shape_token, i),
        }
    }

    fn display_name() -> String {
        "type".to_string()
    }

    fn default_error_value() -> Self::Output {
        SyntaxShape::Any
    }
}

pub(crate) struct DoublePoint {}
impl Parse for DoublePoint {
    type Output = ();

    fn parse(tokens: &[Token], i: usize) -> (Self::Output, usize, Option<ParseError>) {
        if is_baseline_token_matching(&tokens[i], ":") {
            ((), i + 1, None)
        } else {
            Self::mismatch_default_return(&tokens[i], i)
        }
    }

    fn display_name() -> String {
        ":".to_string()
    }

    fn default_error_value() -> Self::Output {
        ()
    }
}

pub(crate) struct Comma {}
impl Parse for Comma {
    type Output = ();

    fn parse(tokens: &[Token], i: usize) -> (Self::Output, usize, Option<ParseError>) {
        if is_baseline_token_matching(&tokens[i], ",") {
            ((), i + 1, None)
        } else {
            Self::mismatch_default_return(&tokens[i], i)
        }
    }

    fn display_name() -> String {
        ",".to_string()
    }

    fn default_error_value() -> Self::Output {
        ()
    }
}

pub(crate) struct EOL {}
impl Parse for EOL {
    type Output = ();

    fn parse(tokens: &[Token], i: usize) -> (Self::Output, usize, Option<ParseError>) {
        if tokens[i].contents.is_eol() {
            ((), i + 1, None)
        } else {
            Self::mismatch_default_return(&tokens[i], i)
        }
    }

    fn display_name() -> String {
        "\\n".to_string()
    }

    fn default_error_value() -> Self::Output {
        ()
    }
}

pub(crate) struct Comment {}
impl Parse for Comment {
    type Output = String;

    fn parse(tokens: &[Token], i: usize) -> (Self::Output, usize, Option<ParseError>) {
        if let TokenContents::Comment(comment) = &tokens[i].contents {
            let comment_text = comment.trim().to_string();
            (comment_text, i + 1, None)
        } else {
            Self::mismatch_default_return(&tokens[i], i)
        }
    }

    fn display_name() -> String {
        "#Comment".to_string()
    }

    fn default_error_value() -> Self::Output {
        "".to_string()
    }
}

pub(crate) struct OptionalModifier {}
impl Parse for OptionalModifier {
    type Output = ();

    fn parse(tokens: &[Token], i: usize) -> (Self::Output, usize, Option<ParseError>) {
        if is_baseline_token_matching(&tokens[i], "?") {
            ((), i + 1, None)
        } else {
            Self::mismatch_default_return(&tokens[i], i)
        }
    }

    fn display_name() -> String {
        "optional modifier keyword".to_string()
    }

    fn default_error_value() -> Self::Output {
        ()
    }
}

pub(crate) struct ParameterName {}
impl Parse for ParameterName {
    type Output = String;

    fn parse(tokens: &[Token], i: usize) -> (Self::Output, usize, Option<ParseError>) {
        if let TokenContents::Baseline(name) = &tokens[i].contents {
            (name.clone(), i + 1, None)
        } else {
            Self::mismatch_default_return(&tokens[i], i)
        }
    }

    fn display_name() -> String {
        "parameter name".to_string()
    }

    fn default_error_value() -> Self::Output {
        "InternalError".to_string()
    }
}

pub(crate) struct FlagName {}
impl Parse for FlagName {
    type Output = String;

    fn parse(tokens: &[Token], i: usize) -> (Self::Output, usize, Option<ParseError>) {
        let flag_token = &tokens[i];
        if let TokenContents::Baseline(name) = &flag_token.contents {
            if !name.starts_with("--") {
                (
                    //Okay return name as flag name eventhough it does not start with --
                    name.clone(),
                    i + 1,
                    Some(ParseError::mismatch(
                        "longform of a flag (Starting with --)",
                        token_to_spanned_string(flag_token),
                    )),
                )
            } else {
                //Discard preceding --
                let name = name[2..].to_string();
                (name.clone(), i + 1, None)
            }
        } else {
            Self::mismatch_default_return(flag_token, i)
        }
    }

    fn display_name() -> String {
        "flag name".to_string()
    }

    fn default_error_value() -> Self::Output {
        "InternalError".to_string()
    }
}

pub(crate) struct FlagShortName {}
impl Parse for FlagShortName {
    type Output = char;

    fn parse(tokens: &[Token], i: usize) -> (Self::Output, usize, Option<ParseError>) {
        let flag_token = &tokens[i];
        return if let TokenContents::Baseline(shortform) = &flag_token.contents {
            let mut chars = shortform.chars();
            match (chars.next(), chars.next_back()) {
                (Some('('), Some(')')) => {
                    let mut err = None;

                    let flag_span = Span::new(
                        flag_token.span.start() + 1, //Skip '('
                        flag_token.span.end() - 1,   // Skip ')'
                    );

                    let c: String = chars.collect();
                    let dash_count = c.chars().take_while(|c| *c == '-').count();
                    err = err.or_else(|| {
                        err_on_too_many_dashes(dash_count, c.clone().spanned(flag_span))
                    });
                    let name = &c[dash_count..];
                    err = err.or_else(|| err_on_name_too_long(name, c.clone().spanned(flag_span)));
                    let c = name.chars().next();

                    (c.unwrap_or(Self::default_error_value()), i + 1, err)
                }
                _ => Self::mismatch_default_return(flag_token, i),
            }
        } else {
            Self::mismatch_default_return(flag_token, i)
        };

        fn err_on_too_many_dashes(
            dash_count: usize,
            actual: Spanned<String>,
        ) -> Option<ParseError> {
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

    fn display_name() -> String {
        "flag shortname".to_string()
    }

    fn default_error_value() -> Self::Output {
        'E'
    }
}

pub(crate) struct RestName {}
impl Parse for RestName {
    type Output = bool;

    fn parse(tokens: &[Token], i: usize) -> (Self::Output, usize, Option<ParseError>) {
        let name_token = &tokens[i];
        return if let TokenContents::Baseline(name) = &name_token.contents {
            if !name.starts_with("...") {
                //Don't parse this token as rest token
                Self::mismatch_default_return(name_token, i)
            } else if !name.starts_with("...rest") || name.len() != "...rest".len() {
                //Okay accept this as rest, but give user warning
                (true, i + 1, rest_name_must_be_rest_error(name_token))
            } else {
                //Okay correct name
                (true, i + 1, None)
            }
        } else {
            Self::mismatch_default_return(name_token, i)
        };

        fn rest_name_must_be_rest_error(token: &Token) -> Option<ParseError> {
            Some(ParseError::mismatch(
                "rest argument name to be 'rest'",
                token_to_spanned_string(token),
            ))
        }
    }

    fn display_name() -> String {
        "rest name".to_string()
    }

    fn default_error_value() -> Self::Output {
        false
    }
}

fn is_baseline_token_matching(token: &Token, string: &str) -> bool {
    match &token.contents {
        TokenContents::Baseline(base) => base == string,
        _ => false,
    }
}
