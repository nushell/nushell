use crate::{
    lex::{Token, TokenContents},
    parse::util::token_to_spanned_string,
};
use nu_errors::ParseError;
use nu_errors::ParseWarning;
use nu_protocol::SyntaxShape;
use nu_source::{Span, Spanned, SpannedItem};

use super::lib_code::{
    parse_lib::{Expect, Parse},
    ParseResult,
};

pub(crate) struct ShapeUnchecked;
pub(crate) type Shape = Expect<ShapeUnchecked>;
impl Parse for ShapeUnchecked {
    type Output = SyntaxShape;

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        let shape_token = &tokens[i];
        match &shape_token.contents {
            TokenContents::Baseline(type_str) => {
                let (shape, err) = match type_str.as_str() {
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
                        Self::default_error_value(),
                        Self::mismatch_error(shape_token),
                    ),
                };

                ParseResult::new(shape, i + 1, err, vec![])
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

pub(crate) struct DoublePointUnchecked {}
pub(crate) type DoublePoint = Expect<DoublePointUnchecked>;
impl Parse for DoublePointUnchecked {
    type Output = ();

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        if is_baseline_token_matching(&tokens[i], ":") {
            ParseResult::new((), i + 1, None, vec![])
        } else {
            Self::mismatch_default_return(&tokens[i], i)
        }
    }

    fn display_name() -> String {
        ":".to_string()
    }

    fn default_error_value() -> Self::Output {}
}

pub(crate) struct CommaUnchecked {}
pub(crate) type Comma = Expect<CommaUnchecked>;
impl Parse for CommaUnchecked {
    type Output = ();

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        if is_baseline_token_matching(&tokens[i], ",") {
            ((), i + 1).into()
        } else {
            Self::mismatch_default_return(&tokens[i], i)
        }
    }

    fn display_name() -> String {
        ",".to_string()
    }

    fn default_error_value() -> Self::Output {}
}

pub(crate) struct EOLUnchecked {}
pub(crate) type EOL = Expect<EOLUnchecked>;
impl Parse for EOLUnchecked {
    type Output = ();

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        if tokens[i].contents.is_eol() {
            ((), i + 1).into()
        } else {
            Self::mismatch_default_return(&tokens[i], i)
        }
    }

    fn display_name() -> String {
        "\\n".to_string()
    }

    fn default_error_value() -> Self::Output {}
}

pub(crate) struct CommentUnchecked {}
pub(crate) type Comment = Expect<CommentUnchecked>;
impl Parse for CommentUnchecked {
    type Output = String;

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        if let TokenContents::Comment(comment) = &tokens[i].contents {
            let comment_text = comment.trim().to_string();
            (comment_text, i + 1).into()
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

pub(crate) struct OptionalModifierUnchecked {}
pub(crate) type OptionalModifier = Expect<OptionalModifierUnchecked>;
impl Parse for OptionalModifierUnchecked {
    type Output = ();

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        if is_baseline_token_matching(&tokens[i], "?") {
            ((), i + 1).into()
        } else {
            Self::mismatch_default_return(&tokens[i], i)
        }
    }

    fn display_name() -> String {
        "optional modifier keyword".to_string()
    }

    fn default_error_value() -> Self::Output {}
}

pub(crate) struct ParameterNameUnchecked {}
pub(crate) type ParameterName = Expect<ParameterNameUnchecked>;

impl Parse for ParameterNameUnchecked {
    type Output = String;

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        if let TokenContents::Baseline(name) = &tokens[i].contents {
            (name.clone(), i + 1).into()
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

pub(crate) struct FlagNameUnchecked {}
pub(crate) type FlagName = Expect<FlagNameUnchecked>;
impl Parse for FlagNameUnchecked {
    type Output = String;

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        let flag_token = &tokens[i];
        if let TokenContents::Baseline(name) = &flag_token.contents {
            if !name.starts_with("--") {
                ParseResult::new(
                    name.clone(),
                    i,
                    Some(ParseError::mismatch(
                        "longform of a flag (Starting with --)",
                        token_to_spanned_string(flag_token),
                    )),
                    vec![],
                )
            } else {
                //Discard preceding --
                let name = name[2..].to_string();
                ParseResult::new(name, i + 1, None, vec![])
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

pub(crate) struct FlagShortNameUnchecked {}
pub(crate) type FlagShortName = Expect<FlagShortNameUnchecked>;
impl Parse for FlagShortNameUnchecked {
    type Output = char;

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        let flag_token = &tokens[i];
        return if let TokenContents::Baseline(shortform) = &flag_token.contents {
            let mut chars = shortform.chars();
            match (chars.next(), chars.next_back()) {
                (Some('('), Some(')')) => {
                    let flag_span = Span::new(
                        flag_token.span.start() + 1, //Skip '('
                        flag_token.span.end() - 1,   // Skip ')'
                    );

                    let shortform_flag: String = chars.collect();
                    let dash_count = shortform_flag.chars().take_while(|c| *c == '-').count();

                    let name = &shortform_flag[dash_count..];

                    let c = name
                        .chars()
                        .next()
                        .unwrap_or_else(Self::default_error_value);

                    //Check for warnings
                    let warnings = vec![
                        warning_on_wrong_dash_count(
                            dash_count,
                            shortform_flag.clone().spanned(flag_span),
                        ),
                        warning_on_wrong_name_len(name, shortform_flag.clone().spanned(flag_span)),
                    ]
                    .into_iter()
                    .filter_map(|e| e)
                    .collect::<Vec<_>>();

                    ParseResult::new(c, i + 1, None, warnings)
                }
                _ => Self::mismatch_default_return(flag_token, i),
            }
        } else {
            Self::mismatch_default_return(flag_token, i)
        };

        fn warning_on_wrong_dash_count(
            dash_count: usize,
            actual: Spanned<String>,
        ) -> Option<ParseWarning> {
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

        fn warning_on_wrong_name_len(name: &str, actual: Spanned<String>) -> Option<ParseError> {
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

pub(crate) struct RestNameUnchecked {}
pub(crate) type RestName = Expect<RestNameUnchecked>;
impl Parse for RestNameUnchecked {
    type Output = bool;

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        let name_token = &tokens[i];
        return if let TokenContents::Baseline(name) = &name_token.contents {
            if !name.starts_with("...") {
                //Don't parse this token as rest token
                Self::mismatch_default_return(name_token, i)
            } else if !name.starts_with("...rest") || name.len() != "...rest".len() {
                //Okay accept this as rest, but give user warning
                ParseResult::new(
                    true,
                    i + 1,
                    None,
                    vec![warning_rest_name_must_be_rest(name_token)],
                )
            } else {
                //Okay correct name
                ParseResult::new(true, i + 1, None, vec![])
            }
        } else {
            Self::mismatch_default_return(name_token, i)
        };

        fn warning_rest_name_must_be_rest(token: &Token) -> ParseError {
            ParseError::mismatch(
                "rest argument name to be 'rest'",
                token_to_spanned_string(token),
            )
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
