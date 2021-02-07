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

use nu_errors::ParseError;
use nu_protocol::{NamedType, PositionalType, Signature, SyntaxShape};
use nu_source::{Span, Spanned};

use crate::lex::{
    lexer::{lex, Token},
    tokens::TokenContents,
};

use super::{
    data_structs::{Description, Flag, Parameter},
    primitives::{
        is_baseline_token_matching, parse_comma, parse_eol, parse_flag_name,
        parse_flag_optional_shortform, parse_optional_comment,
        parse_optional_parameter_optional_modifier, parse_param_name, parse_rest_name,
        parse_type_token,
    },
};

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

    (Some((type_, comment)), i, err)
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
