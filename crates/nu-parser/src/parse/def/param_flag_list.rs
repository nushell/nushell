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
    parse::def::parse_lib::{AndThen, IfSuccessThen, Maybe, Parse},
};
use nu_errors::ParseError;
use nu_protocol::{NamedType, PositionalType, Signature, SyntaxShape};
use nu_source::{Span, Spanned};

use super::{
    parse_lib::CheckedParse,
    primitives::{
        Comma, Comment, DoublePoint, FlagName, FlagShortName, OptionalModifier, ParameterName,
        RestName, Shape, EOL,
    },
};

pub(crate) fn parse_signature(
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
            let (flag, i_new, error) = Flag::parse_debug(&tokens, i);
            err = err.or(error);
            i = i_new;
            flags.push(flag);
        } else if is_rest(&tokens[i]) {
            let (rest_, i_new, error) = Rest::parse_debug(&tokens, i);
            err = err.or(error);
            i = i_new;
            rest = Some(rest_);
        } else {
            let (parameter, i_new, error) = Parameter::parse_debug(&tokens, i);
            err = err.or(error);
            i = i_new;
            parameters.push(parameter);
        }
    }

    let signature = to_signature(name, parameters, flags, rest);
    debug!("Signature: {:?}", signature);

    (signature, err)
}

impl CheckedParse for Parameter {}
impl Parse for Parameter {
    type Output = Parameter;

    fn parse(tokens: &[Token], i: usize) -> (Self::Output, usize, Option<ParseError>) {
        // let i_start = i;

        let ((name, (optional, (type_, comment))), i, err) = AndThen::<
            ParameterName,
            AndThen<Maybe<OptionalModifier>, AndThen<OptionalType, ItemEnd>>,
        >::parse(tokens, i);

        // let i_end = i;

        let type_ = type_.unwrap_or(SyntaxShape::Any);
        // let span = tokens[i_start].span.until(tokens[i_end - 1].span);
        let span = Span::unknown();

        let pos_type = if optional.is_some() {
            PositionalType::optional(&name, type_)
        } else {
            PositionalType::mandatory(&name, type_)
        };

        let parameter = Parameter::new(pos_type, comment, span);

        debug!(
            "Parsed parameter: {} with shape {:?}",
            parameter.pos_type.name(),
            parameter.pos_type.syntax_type()
        );

        (parameter, i, err)
    }

    fn display_name() -> String {
        "parameter item".to_string()
    }

    fn default_error_value() -> Self::Output {
        Parameter::error()
    }
}

impl CheckedParse for Flag {}
impl Parse for Flag {
    type Output = Flag;

    fn parse(tokens: &[Token], i: usize) -> (Self::Output, usize, Option<ParseError>) {
        // let i_start = i;

        let ((name, (shortform, (type_, comment))), i, err) = AndThen::<
            FlagName,
            AndThen<Maybe<FlagShortName>, AndThen<OptionalType, ItemEnd>>,
        >::parse(tokens, i);

        // let i_end = i;
        // let span = tokens[i_start].span.until(tokens[i_end - 1].span);
        let span = Span::unknown();

        //If no type is given, the flag is a switch. Otherwise its optional
        //Example:
        //--verbose(-v) # Switch
        //--output(-o): path # Optional flag
        let named_type = if let Some(shape) = type_ {
            NamedType::Optional(shortform, shape)
        } else {
            NamedType::Switch(shortform)
        };

        let flag = Flag::new(name, named_type, comment, span);

        debug!("Parsed flag: {:?}", flag);
        (flag, i, err)
    }

    fn display_name() -> String {
        "Flag item".to_string()
    }

    fn default_error_value() -> Self::Output {
        Flag::error()
    }
}

struct Rest;
impl CheckedParse for Rest {}
impl Parse for Rest {
    type Output = (SyntaxShape, Description);

    fn parse(tokens: &[Token], i: usize) -> (Self::Output, usize, Option<ParseError>) {
        let ((_, (type_, comment)), i, err) =
            AndThen::<RestName, AndThen<OptionalType, ItemEnd>>::parse(tokens, i);

        return (
            (
                type_.unwrap_or(SyntaxShape::Any),
                comment.unwrap_or("".to_string()),
            ),
            i,
            err,
        );
    }

    fn display_name() -> String {
        "Rest item".to_string()
    }

    fn default_error_value() -> Self::Output {
        (SyntaxShape::Any, "".to_string())
    }
}

///Parses the end of a flag or a parameter
///Return value is Option<Comment>
///   (<,>)? (#Comment)? (<eol>)?
// type ItemEnd = Option<Description>;
struct ItemEnd {}
impl CheckedParse for ItemEnd {}
impl Parse for ItemEnd {
    //Item end Output is optional Comment
    type Output = Option<Description>;
    fn parse(tokens: &[Token], i: usize) -> (Self::Output, usize, Option<ParseError>) {
        let ((_, (comment, _)), i, err) =
            AndThen::<Maybe<Comma>, AndThen<Maybe<Comment>, Maybe<EOL>>>::parse(tokens, i);

        (comment, i, err)
    }

    fn display_name() -> String {
        "item end".to_string()
    }

    fn default_error_value() -> Self::Output {
        None
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

//Currently the lexer does not split off baselines after existing text
//Example --flag(-f) is lexed as one baseline token.
//To properly parse the input, it is required that --flag and (-f) are 2 tokens.
pub(crate) fn lex_split_shortflag_from_longflag(tokens: Vec<Token>) -> Vec<Token> {
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
pub(crate) fn lex_split_baseline_tokens_on(
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
pub(crate) struct Flag {
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

struct OptionalType {}
impl CheckedParse for OptionalType {}

impl Parse for OptionalType {
    type Output = Option<SyntaxShape>;

    fn parse(tokens: &[Token], i: usize) -> (Self::Output, usize, Option<ParseError>) {
        let (values, i_new, err) = IfSuccessThen::<DoublePoint, Shape>::parse(tokens, i);
        if let Some((_, shape)) = values {
            (Some(shape), i_new, err)
        } else {
            (None, i, None)
        }
    }

    fn display_name() -> String {
        "type".to_string()
    }

    fn default_error_value() -> Self::Output {
        Some(SyntaxShape::Any)
    }
}
