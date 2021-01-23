///This module contains functions to parse the parameter and flag list (signature)
///Such a signature can be of the following format:
/// [ (parameter | flag | rest_param | <eol>)* ]
///Where
///parameter is:
///    name (<:> type)? (<?>)? item_end
///flag is:
///    --name (-shortform)? (<:> type)? item_end
///rest is:
///    ...rest (<:> type)? item_end
///item_end:
///    (<,>)? (#Comment)? (<eol>)?
///
use log::debug;

use crate::{
    lex::{lex, Token, TokenContents},
    parse::util::token_to_spanned_string,
};
use nu_errors::ParseError;
use nu_protocol::{NamedType, PositionalType, Signature, SyntaxShape};
use nu_source::{Span, Spanned, SpannedItem};

pub fn parse_signature(
    name: &str,
    signature_vec: &Spanned<String>,
) -> (Signature, Option<ParseError>) {
    let mut err = None;

    let mut chars = signature_vec.chars();

    match (chars.next(), chars.next_back()) {
        (Some('['), Some(']')) => {}
        _ => {
            err = err.or_else(|| {
                Some(ParseError::mismatch(
                    "definition signature",
                    signature_vec.clone(),
                ))
            });
        }
    }

    let string: String = chars.collect();

    debug!(
        "signature vec span start: {}",
        signature_vec.span.start() + 1
    );
    let (tokens, error) = lex(&string, signature_vec.span.start() + 1);
    err = err.or(error);

    //After normal lexing, tokens also need to be split on ',' and ':'
    //TODO this could probably be all done in a specialized lexing function
    let tokens = lex_split_baseline_tokens_on(tokens, &[',', ':', '?']);
    let tokens = lex_split_shortflag_from_longflag(tokens);
    debug!("Tokens are {:?}", tokens);

    let mut parameters = vec![];
    let mut flags = vec![];
    let mut rest = None;
    let mut i = 0;

    while i < tokens.len() {
        if tokens[i].contents.is_eol() {
            //Skip leading eol
            i += 1;
        } else if is_flag(&tokens[i]) {
            let (flag, advanced_by, error) = parse_flag(&tokens[i..], signature_vec);
            err = err.or(error);
            i += advanced_by;
            flags.push(flag);
        } else if is_rest(&tokens[i]) {
            let (rest_, advanced_by, error) = parse_rest(&tokens[i..], signature_vec);
            err = err.or(error);
            i += advanced_by;
            rest = rest_;
        } else {
            let (parameter, advanced_by, error) = parse_parameter(&tokens[i..], signature_vec);
            err = err.or(error);
            i += advanced_by;
            parameters.push(parameter);
        }
    }

    let signature = to_signature(name, parameters, flags, rest);
    debug!("Signature: {:?}", signature);

    (signature, err)
}

fn parse_parameter(
    tokens: &[Token],
    tokens_as_str: &Spanned<String>,
) -> (Parameter, usize, Option<ParseError>) {
    if tokens.is_empty() {
        //TODO fix span
        return (
            Parameter::error(),
            0,
            Some(ParseError::unexpected_eof("parameter", tokens_as_str.span)),
        );
    }

    let mut err: Option<ParseError> = None;
    let mut i = 0;
    let mut type_ = SyntaxShape::Any;
    let mut comment = None;
    let mut optional = false;

    let (name, error) = parse_param_name(&tokens[0]);
    i += 1;
    err = err.or(error);

    if i < tokens.len() {
        let (parsed_opt_modifier, advanced_by) =
            parse_optional_parameter_optional_modifier(&tokens[i]);
        optional = parsed_opt_modifier;
        i += advanced_by;
    }

    if i < tokens.len() {
        let (parsed_type_, advanced_by, error) = parse_optional_type(&tokens[i..]);
        type_ = parsed_type_.unwrap_or(SyntaxShape::Any);
        err = err.or(error);
        i += advanced_by;
    }

    if i < tokens.len() {
        let (comment_text, advanced_by, error) = parse_signature_item_end(&tokens[i..]);
        comment = comment_text;
        i += advanced_by;
        err = err.or(error);
    }

    let pos_type = if optional {
        PositionalType::optional(&name.item, type_)
    } else {
        PositionalType::mandatory(&name.item, type_)
    };

    let parameter = Parameter::new(pos_type, comment, name.span);

    debug!(
        "Parsed parameter: {} with shape {:?}",
        parameter.pos_type.name(),
        parameter.pos_type.syntax_type()
    );

    (parameter, i, err)
}

fn parse_flag(
    tokens: &[Token],
    tokens_as_str: &Spanned<String>,
) -> (Flag, usize, Option<ParseError>) {
    if tokens.is_empty() {
        return (
            Flag::error(),
            0,
            Some(ParseError::unexpected_eof("parameter", tokens_as_str.span)),
        );
    }

    let mut err: Option<ParseError> = None;
    let mut i = 0;
    let mut shortform = None;
    let mut type_ = None;
    let mut comment = None;

    let (name, error) = parse_flag_name(&tokens[0]);
    err = err.or(error);
    i += 1;

    if i < tokens.len() {
        let (parsed_shortform, advanced_by, error) = parse_flag_optional_shortform(&tokens[i..]);
        shortform = parsed_shortform;
        i += advanced_by;
        err = err.or(error);
    }

    if i < tokens.len() {
        let (parsed_type, advanced_by, error) = parse_optional_type(&tokens[i..]);
        type_ = parsed_type;
        i += advanced_by;
        err = err.or(error);
    }

    if i < tokens.len() {
        let (parsed_comment, advanced_by, error) = parse_signature_item_end(&tokens[i..]);
        comment = parsed_comment;
        i += advanced_by;
        err = err.or(error);
    }

    //If no type is given, the flag is a switch. Otherwise its optional
    //Example:
    //--verbose(-v) # Switch
    //--output(-o): path # Optional flag
    let named_type = if let Some(shape) = type_ {
        NamedType::Optional(shortform, shape)
    } else {
        NamedType::Switch(shortform)
    };

    let flag = Flag::new(name.item.clone(), named_type, comment, name.span);

    debug!("Parsed flag: {:?}", flag);
    (flag, i, err)
}

fn parse_rest(
    tokens: &[Token],
    tokens_as_str: &Spanned<String>,
) -> (
    Option<(SyntaxShape, Description)>,
    usize,
    Option<ParseError>,
) {
    if tokens.is_empty() {
        return (
            None,
            0,
            Some(ParseError::unexpected_eof(
                "rest argument",
                tokens_as_str.span,
            )),
        );
    }

    let mut err = None;
    let mut i = 0;
    let mut type_ = SyntaxShape::Any;
    let mut comment = "".to_string();

    let error = parse_rest_name(&tokens[i]);
    err = err.or(error);
    i += 1;

    if i < tokens.len() {
        let (parsed_type, advanced_by, error) = parse_optional_type(&tokens[i..]);
        err = err.or(error);
        i += advanced_by;
        type_ = parsed_type.unwrap_or(SyntaxShape::Any);
    }

    if i < tokens.len() {
        let (parsed_comment, advanced_by) = parse_optional_comment(&tokens[i..]);
        i += advanced_by;
        comment = parsed_comment.unwrap_or_else(|| "".to_string());
    }

    return (Some((type_, comment)), i, err);

    fn parse_rest_name(name_token: &Token) -> Option<ParseError> {
        return if let TokenContents::Baseline(name) = &name_token.contents {
            if !name.starts_with("...") {
                parse_rest_name_err(name_token)
            } else if !name.starts_with("...rest") {
                Some(ParseError::mismatch(
                    "rest argument name to be 'rest'",
                    token_to_spanned_string(name_token),
                ))
            } else {
                None
            }
        } else {
            parse_rest_name_err(name_token)
        };

        fn parse_rest_name_err(token: &Token) -> Option<ParseError> {
            Some(ParseError::mismatch(
                "...rest",
                token_to_spanned_string(token),
            ))
        }
    }
}

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

fn parse_type_token(type_: &Token) -> (SyntaxShape, Option<ParseError>) {
    match &type_.contents {
        TokenContents::Baseline(type_str) => parse_type(&type_str.clone().spanned(type_.span)),
        _ => (
            SyntaxShape::Any,
            Some(ParseError::mismatch(
                "type",
                type_.contents.to_string().spanned(type_.span),
            )),
        ),
    }
}

fn parse_param_name(token: &Token) -> (Spanned<String>, Option<ParseError>) {
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

fn parse_optional_comment(tokens: &[Token]) -> (Option<String>, usize) {
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

fn parse_optional_type(tokens: &[Token]) -> (Option<SyntaxShape>, usize, Option<ParseError>) {
    fn is_double_point(token: &Token) -> bool {
        is_baseline_token_matching(token, ":")
    }
    let mut err = None;
    let mut type_ = None;
    let mut i: usize = 0;
    //Check if a type has to follow
    if i < tokens.len() && is_double_point(&tokens[i]) {
        //Type has to follow
        if i + 1 == tokens.len() {
            err = err.or_else(|| Some(ParseError::unexpected_eof("type", tokens[i].span)));
        } else {
            //Jump over <:>
            i += 1;
            let (shape, error) = parse_type_token(&tokens[i]);
            err = err.or(error);
            type_ = Some(shape);
            i += 1;
        }
    }
    (type_, i, err)
}

///Parse token if it is a modifier to make
fn parse_optional_parameter_optional_modifier(token: &Token) -> (bool, usize) {
    fn is_questionmark(token: &Token) -> bool {
        is_baseline_token_matching(token, "?")
    }
    if is_questionmark(token) {
        (true, 1)
    } else {
        (false, 0)
    }
}

///Parses the end of a flag or a parameter
///   (<,>)? (#Comment)? (<eol>)?
fn parse_signature_item_end(tokens: &[Token]) -> (Option<String>, usize, Option<ParseError>) {
    if tokens.is_empty() {
        //If no more tokens, parameter/flag doesn't need ',' or comment to be properly finished
        return (None, 0, None);
    }

    let mut i = 0;
    let err = None;
    let (parsed_comma, advanced_by) = parse_comma(&tokens[i..]);
    i += advanced_by;
    let (comment, advanced_by) = parse_optional_comment(&tokens[i..]);
    i += advanced_by;
    let (parsed_eol, advanced_by) = parse_eol(&tokens[i..]);
    i += advanced_by;

    debug!(
        "Parsed comma {} and parsed eol {}",
        parsed_comma, parsed_eol
    );
    ////Separating flags/parameters is optional.
    ////If this should change, the below code would raise a warning whenever 2 parameters/flags are
    ////not delmited by <,> or <eol>
    //if there is next item, but it's not comma, then it must be Optional(#Comment) + <eof>
    //let parsed_delimiter = parsed_comma || parsed_eol;
    //if !parsed_delimiter && i < tokens.len() {
    //    //If not parsed , or eol but more tokens are comming
    //    err = err.or(Some(ParseError::mismatch(
    //        "Newline or ','",
    //        (token[i-1].to_string() + token[i].to_string()).spanned(token[i-1].span.until(token[i].span))
    //    )));
    //}

    (comment, i, err)
}

fn parse_flag_name(token: &Token) -> (Spanned<String>, Option<ParseError>) {
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

fn parse_flag_optional_shortform(tokens: &[Token]) -> (Option<char>, usize, Option<ParseError>) {
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

fn parse_eol(tokens: &[Token]) -> (bool, usize) {
    if !tokens.is_empty() && tokens[0].contents.is_eol() {
        (true, 1)
    } else {
        (false, 0)
    }
}

fn parse_comma(tokens: &[Token]) -> (bool, usize) {
    fn is_comma(token: &Token) -> bool {
        is_baseline_token_matching(token, ",")
    }
    if !tokens.is_empty() && is_comma(&tokens[0]) {
        (true, 1)
    } else {
        (false, 0)
    }
}

///Returns true if token potentially represents rest argument
fn is_rest(token: &Token) -> bool {
    match &token.contents {
        TokenContents::Baseline(item) => item.starts_with("..."),
        _ => false,
    }
}

///True for short or longform flags. False otherwise
fn is_flag(token: &Token) -> bool {
    match &token.contents {
        TokenContents::Baseline(item) => item.starts_with('-'),
        _ => false,
    }
}

fn to_signature(
    name: &str,
    params: Vec<Parameter>,
    flags: Vec<Flag>,
    rest: Option<(SyntaxShape, Description)>,
) -> Signature {
    let mut sign = Signature::new(name);

    for param in params.into_iter() {
        // pub positional: Vec<(PositionalType, Description)>,
        sign.positional
            .push((param.pos_type, param.desc.unwrap_or_else(|| "".to_string())));
    }

    for flag in flags.into_iter() {
        sign.named.insert(
            flag.long_name,
            (flag.named_type, flag.desc.unwrap_or_else(|| "".to_string())),
        );
    }

    sign.rest_positional = rest;

    sign
}

fn is_baseline_token_matching(token: &Token, string: &str) -> bool {
    match &token.contents {
        TokenContents::Baseline(base) => base == string,
        _ => false,
    }
}

//Currently the lexer does not split off baselines after existing text
//Example --flag(-f) is lexed as one baseline token.
//To properly parse the input, it is required that --flag and (-f) are 2 tokens.
fn lex_split_shortflag_from_longflag(tokens: Vec<Token>) -> Vec<Token> {
    let mut result = Vec::with_capacity(tokens.capacity());
    for token in tokens {
        let mut processed = false;
        if let TokenContents::Baseline(base) = &token.contents {
            if let Some(paren_start) = base.find('(') {
                if paren_start != 0 {
                    processed = true;
                    //If token contains '(' and '(' is not the first char,
                    //we split on '('
                    //Example: Baseline(--flag(-f)) results in: [Baseline(--flag), Baseline((-f))]
                    let paren_span_i = token.span.start() + paren_start;
                    result.push(Token::new(
                        TokenContents::Baseline(base[..paren_start].to_string()),
                        Span::new(token.span.start(), paren_span_i),
                    ));
                    result.push(Token::new(
                        TokenContents::Baseline(base[paren_start..].to_string()),
                        Span::new(paren_span_i, token.span.end()),
                    ));
                }
            }
        }

        if !processed {
            result.push(token);
        }
    }
    result
}
//Currently the lexer does not split baselines on ',' ':' '?'
//The parameter list requires this. Therefore here is a hacky method doing this.
fn lex_split_baseline_tokens_on(
    tokens: Vec<Token>,
    extra_baseline_terminal_tokens: &[char],
) -> Vec<Token> {
    debug!("Before lex fix up {:?}", tokens);
    let make_new_token =
        |token_new: String, token_new_end: usize, terminator_char: Option<char>| {
            let end = token_new_end;
            let start = end - token_new.len();

            let mut result = vec![];
            //Only add token if its not empty
            if !token_new.is_empty() {
                result.push(Token::new(
                    TokenContents::Baseline(token_new),
                    Span::new(start, end),
                ));
            }
            //Insert terminator_char as baseline token
            if let Some(ch) = terminator_char {
                result.push(Token::new(
                    TokenContents::Baseline(ch.to_string()),
                    Span::new(end, end + 1),
                ));
            }

            result
        };
    let mut result = Vec::with_capacity(tokens.len());
    for token in tokens {
        match token.contents {
            TokenContents::Baseline(base) => {
                let token_offset = token.span.start();
                let mut current = "".to_string();
                for (i, c) in base.chars().enumerate() {
                    if extra_baseline_terminal_tokens.contains(&c) {
                        result.extend(make_new_token(current, i + token_offset, Some(c)));
                        current = "".to_string();
                    } else {
                        current.push(c);
                    }
                }
                result.extend(make_new_token(current, base.len() + token_offset, None));
            }
            _ => result.push(token),
        }
    }
    result
}

type Description = String;
#[derive(Clone)]
struct Parameter {
    pub pos_type: PositionalType,
    pub desc: Option<Description>,
    pub span: Span,
}

impl Parameter {
    pub fn new(pos_type: PositionalType, desc: Option<Description>, span: Span) -> Parameter {
        Parameter {
            pos_type,
            desc,
            span,
        }
    }

    pub fn error() -> Parameter {
        Parameter::new(
            PositionalType::optional("Internal Error", SyntaxShape::Any),
            Some(
                "Wanted to parse a parameter, but no input present. Please report this error!"
                    .to_string(),
            ),
            Span::unknown(),
        )
    }
}

#[derive(Clone, Debug)]
struct Flag {
    pub long_name: String,
    pub named_type: NamedType,
    pub desc: Option<Description>,
    pub span: Span,
}

impl Flag {
    pub fn new(
        long_name: String,
        named_type: NamedType,
        desc: Option<Description>,
        span: Span,
    ) -> Flag {
        Flag {
            long_name,
            named_type,
            desc,
            span,
        }
    }

    pub fn error() -> Flag {
        Flag::new(
            "Internal Error".to_string(),
            NamedType::Switch(None),
            Some(
                "Wanted to parse a flag, but no input present. Please report this error!"
                    .to_string(),
            ),
            Span::unknown(),
        )
    }
}

mod tests {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use nu_test_support::nu;

    #[test]
    fn simple_def_with_params() {
        let name = "my_func";
        let sign = "[param1?: int, param2: string]";
        let (sign, err) = parse_signature(name, &sign.to_string().spanned(Span::new(0, 27)));
        assert!(err.is_none());
        assert_eq!(
            sign.positional,
            vec![
                (
                    PositionalType::Optional("param1".into(), SyntaxShape::Int),
                    "".into()
                ),
                (
                    PositionalType::Mandatory("param2".into(), SyntaxShape::String),
                    "".into()
                ),
            ]
        );
    }

    #[test]
    fn simple_def_with_optional_param_without_type() {
        let name = "my_func";
        let sign = "[param1 ?, param2?]";
        let (sign, err) = parse_signature(name, &sign.to_string().spanned(Span::new(0, 27)));
        assert!(err.is_none());
        assert_eq!(
            sign.positional,
            vec![
                (
                    PositionalType::Optional("param1".into(), SyntaxShape::Any),
                    "".into()
                ),
                (
                    PositionalType::Optional("param2".into(), SyntaxShape::Any),
                    "".into()
                ),
            ]
        );
    }

    #[test]
    fn simple_def_with_params_with_comment() {
        let name = "my_func";
        let sign = "[
        param1:path # My first param
        param2:number # My second param
        ]";
        let (sign, err) = parse_signature(name, &sign.to_string().spanned(Span::new(0, 64)));
        assert!(err.is_none());
        assert_eq!(
            sign.positional,
            vec![
                (
                    PositionalType::Mandatory("param1".into(), SyntaxShape::FilePath),
                    "My first param".into()
                ),
                (
                    PositionalType::Mandatory("param2".into(), SyntaxShape::Number),
                    "My second param".into()
                ),
            ]
        );
    }

    #[test]
    fn simple_def_with_params_without_type() {
        let name = "my_func";
        let sign = "[
        param1 # My first param
        param2:number # My second param
        ]";
        let (sign, err) = parse_signature(name, &sign.to_string().spanned(Span::new(0, 0)));
        assert!(err.is_none());
        assert_eq!(
            sign.positional,
            vec![
                (
                    PositionalType::Mandatory("param1".into(), SyntaxShape::Any),
                    "My first param".into()
                ),
                (
                    PositionalType::Mandatory("param2".into(), SyntaxShape::Number),
                    "My second param".into()
                ),
            ]
        );
    }

    #[test]
    fn oddly_but_correct_written_params() {
        let name = "my_func";
        let sign = "[
        param1 :int         #      param1

        param2 : number # My second param


        param4, param5:path  ,  param6 # param6
        ]";
        let (sign, err) = parse_signature(name, &sign.to_string().spanned(Span::new(0, 0)));
        assert!(err.is_none());
        assert_eq!(
            sign.positional,
            vec![
                (
                    PositionalType::Mandatory("param1".into(), SyntaxShape::Int),
                    "param1".into()
                ),
                (
                    PositionalType::Mandatory("param2".into(), SyntaxShape::Number),
                    "My second param".into()
                ),
                (
                    PositionalType::Mandatory("param4".into(), SyntaxShape::Any),
                    "".into()
                ),
                (
                    PositionalType::Mandatory("param5".into(), SyntaxShape::FilePath),
                    "".into()
                ),
                (
                    PositionalType::Mandatory("param6".into(), SyntaxShape::Any),
                    "param6".into()
                ),
            ]
        );
    }

    #[test]
    fn err_wrong_type() {
        let actual = nu!(
            cwd: ".",
            "def f [ param1:strig ] { echo hi }"
        );
        assert!(actual.err.contains("type"));
    }

    //For what ever reason, this gets reported as not used
    #[allow(dead_code)]
    fn assert_signature_has_flag(sign: &Signature, name: &str, type_: NamedType, comment: &str) {
        assert_eq!(
            Some((type_, comment.to_string())),
            sign.named.get(name).cloned()
        );
    }

    #[test]
    fn simple_def_with_only_flags() {
        let name = "my_func";
        let sign = "[
        --list (-l) : path  # First flag
        --verbose : number # Second flag
        --all(-a) # My switch
        ]";
        let (sign, err) = parse_signature(name, &sign.to_string().spanned_unknown());
        assert!(err.is_none());
        assert_signature_has_flag(
            &sign,
            "list",
            NamedType::Optional(Some('l'), SyntaxShape::FilePath),
            "First flag",
        );
        assert_signature_has_flag(
            &sign,
            "verbose",
            NamedType::Optional(None, SyntaxShape::Number),
            "Second flag",
        );
        assert_signature_has_flag(&sign, "all", NamedType::Switch(Some('a')), "My switch");
    }

    #[test]
    fn simple_def_with_params_and_flags() {
        let name = "my_func";
        let sign = "[
        --list (-l) : path  # First flag
        param1, param2:table # Param2 Doc
        --verbose # Second flag
        param3 : number,
        --flag3 # Third flag
        param4 ?: table # Optional Param
        ]";
        let (sign, err) = parse_signature(name, &sign.to_string().spanned_unknown());
        assert!(err.is_none());
        assert_signature_has_flag(
            &sign,
            "list",
            NamedType::Optional(Some('l'), SyntaxShape::FilePath),
            "First flag",
        );
        assert_signature_has_flag(&sign, "verbose", NamedType::Switch(None), "Second flag");
        assert_signature_has_flag(&sign, "flag3", NamedType::Switch(None), "Third flag");
        assert_eq!(
            sign.positional,
            vec![
                (
                    PositionalType::Mandatory("param1".into(), SyntaxShape::Any),
                    "".into()
                ),
                (
                    PositionalType::Mandatory("param2".into(), SyntaxShape::Table),
                    "Param2 Doc".into()
                ),
                (
                    PositionalType::Mandatory("param3".into(), SyntaxShape::Number),
                    "".into()
                ),
                (
                    PositionalType::Optional("param4".into(), SyntaxShape::Table),
                    "Optional Param".into()
                ),
            ]
        );
    }

    #[test]
    fn simple_def_with_parameters_and_flags_no_delimiter() {
        let name = "my_func";
        let sign = "[ param1:int param2
            --force (-f) param3 # Param3
            ]";
        let (sign, err) = parse_signature(name, &sign.to_string().spanned_unknown());
        assert!(err.is_none());
        assert_signature_has_flag(&sign, "force", NamedType::Switch(Some('f')), "");
        assert_eq!(
            sign.positional,
            // --list (-l) : path  # First flag
            // param1, param2:table # Param2 Doc
            // --verbose # Second flag
            // param3 : number,
            // --flag3 # Third flag
            vec![
                (
                    PositionalType::Mandatory("param1".into(), SyntaxShape::Int),
                    "".into()
                ),
                (
                    PositionalType::Mandatory("param2".into(), SyntaxShape::Any),
                    "".into()
                ),
                (
                    PositionalType::Mandatory("param3".into(), SyntaxShape::Any),
                    "Param3".into()
                ),
            ]
        );
    }

    #[test]
    fn simple_example_signature() {
        let name = "my_func";
        let sign = "[
        d:int          # The required d parameter
        --x (-x):string # The all powerful x flag
        --y (-y):int    # The accompanying y flag
        ]";
        let (sign, err) = parse_signature(name, &sign.to_string().spanned_unknown());
        assert!(err.is_none());
        assert_signature_has_flag(
            &sign,
            "x",
            NamedType::Optional(Some('x'), SyntaxShape::String),
            "The all powerful x flag",
        );
        assert_signature_has_flag(
            &sign,
            "y",
            NamedType::Optional(Some('y'), SyntaxShape::Int),
            "The accompanying y flag",
        );
        assert_eq!(
            sign.positional,
            vec![(
                PositionalType::Mandatory("d".into(), SyntaxShape::Int),
                "The required d parameter".into()
            )]
        );
    }

    #[test]
    fn flag_withouth_space_between_longname_shortname() {
        let name = "my_func";
        let sign = "[
        --xxx(-x):string # The all powerful x flag
        ]";
        let (sign, err) = parse_signature(name, &sign.to_string().spanned_unknown());
        assert!(err.is_none());
        assert_signature_has_flag(
            &sign,
            "xxx",
            NamedType::Optional(Some('x'), SyntaxShape::String),
            "The all powerful x flag",
        );
    }

    #[test]
    fn simple_def_with_rest_arg() {
        let name = "my_func";
        let sign = "[ ...rest]";
        let (sign, err) = parse_signature(name, &sign.to_string().spanned_unknown());
        assert!(err.is_none());
        assert_eq!(
            sign.rest_positional,
            Some((SyntaxShape::Any, "".to_string()))
        );
    }

    #[test]
    fn simple_def_with_rest_arg_with_type_and_comment() {
        let name = "my_func";
        let sign = "[ ...rest:path # My super cool rest arg]";
        let (sign, err) = parse_signature(name, &sign.to_string().spanned_unknown());
        assert!(err.is_none());
        assert_eq!(
            sign.rest_positional,
            Some((SyntaxShape::FilePath, "My super cool rest arg".to_string()))
        );
    }

    #[test]
    fn simple_def_with_param_flag_and_rest() {
        let name = "my_func";
        let sign = "[
        d:string          # The required d parameter
        --xxx(-x)         # The all powerful x flag
        --yyy (-y):int    #    The accompanying y flag
        ...rest:table # Another rest
        ]";
        let (sign, err) = parse_signature(name, &sign.to_string().spanned_unknown());
        assert!(err.is_none());
        assert_signature_has_flag(
            &sign,
            "xxx",
            NamedType::Switch(Some('x')),
            "The all powerful x flag",
        );
        assert_signature_has_flag(
            &sign,
            "yyy",
            NamedType::Optional(Some('y'), SyntaxShape::Int),
            "The accompanying y flag",
        );
        assert_eq!(
            sign.positional,
            vec![(
                PositionalType::Mandatory("d".into(), SyntaxShape::String),
                "The required d parameter".into()
            )]
        );
        assert_eq!(
            sign.rest_positional,
            Some((SyntaxShape::Table, "Another rest".to_string()))
        );
    }
}
