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
use crate::{
    lex::{lex, Token, TokenContents},
    parse::def::lib_code::parse_lib::{And2, CheckedParse, IfSuccessThen, Maybe, Parse},
};
use log::debug;
use nu_errors::ParseError;
use nu_protocol::{NamedType, PositionalType, Signature, SyntaxShape};
use nu_source::{Span, Spanned};

use super::{
    lex_fixup::{lex_split_baseline_tokens_on, lex_split_shortflag_from_longflag},
    lib_code::{
        parse_lib::{And3, WithSpan},
        ParseResult,
    },
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
            let ParseResult {
                value: flag,
                i: i_new,
                err: error,
            } = Flag::parse_debug(&tokens, i);
            err = err.or(error);
            i = i_new;
            flags.push(flag);
        } else if can_be_rest(&tokens[i]) {
            let ParseResult {
                value: rest_,
                i: i_new,
                err: error,
            } = Rest::parse_debug(&tokens, i);
            err = err.or(error);
            i = i_new;
            rest = Some(rest_);
        } else {
            let ParseResult {
                value: parameter,
                i: i_new,
                err: error,
            } = Parameter::parse_debug(&tokens, i);
            err = err.or(error);
            i = i_new;
            parameters.push(parameter);
        }
    }

    let signature = to_signature(name, parameters, flags, rest);
    debug!("Signature: {:?}", signature);

    (signature, err)
}

type Description = String;
#[derive(Clone, new)]
struct Parameter {
    pub pos_type: PositionalType,
    pub desc: Option<Description>,
    pub span: Span,
}

impl CheckedParse for Parameter {}
impl Parse for Parameter {
    type Output = Parameter;

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        let ParseResult {
            value: ((span, (name, optional, type_)), comment),
            i,
            err }
        = And2::<
            WithSpan<And3<ParameterName, Maybe<OptionalModifier>, OptionalType>>,
            ItemEnd,
        >::parse(tokens, i);

        let type_ = type_.unwrap_or(SyntaxShape::Any);

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

        ParseResult::new(parameter, i, err)
    }

    fn display_name() -> String {
        "parameter item".to_string()
    }

    fn default_error_value() -> Self::Output {
        Parameter::new(
            PositionalType::optional("Error", SyntaxShape::Any),
            Some("Garbage parameter, generated from the Parser".to_string()),
            Span::unknown(),
        )
    }
}

#[derive(Clone, Debug, new)]
pub(crate) struct Flag {
    pub long_name: String,
    pub named_type: NamedType,
    pub desc: Option<Description>,
    pub span: Span,
}

impl CheckedParse for Flag {}
impl Parse for Flag {
    type Output = Flag;

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        let ParseResult {
            value: ((span, (name, shortform, type_)), comment),
            i,
            err,
        } = And2::<WithSpan<And3<FlagName, Maybe<FlagShortName>, OptionalType>>, ItemEnd>::parse(
            tokens, i,
        );

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

        ParseResult::new(flag, i, err)
    }

    fn display_name() -> String {
        "flag item".to_string()
    }

    fn default_error_value() -> Self::Output {
        Flag::new(
            "Error".to_string(),
            NamedType::Switch(None),
            Some("Garbage flag, generated from the Parser".to_string()),
            Span::unknown(),
        )
    }
}

struct Rest;
impl CheckedParse for Rest {}
impl Parse for Rest {
    type Output = (SyntaxShape, Description);

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        let ParseResult {
            value: (_, (type_, comment)),
            i,
            err,
        } = And2::<RestName, And2<OptionalType, ItemEnd>>::parse(tokens, i);

        ParseResult::new(
            (
                type_.unwrap_or(SyntaxShape::Any),
                comment.unwrap_or("".to_string()),
            ),
            i,
            err,
        )
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
struct ItemEnd {}
impl CheckedParse for ItemEnd {}
impl Parse for ItemEnd {
    //Item end Output is optional Comment
    type Output = Option<Description>;
    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        let ParseResult {
            value: (_, comment, _),
            i,
            err,
        } = And3::<Maybe<Comma>, Maybe<Comment>, Maybe<EOL>>::parse(tokens, i);

        ParseResult::new(comment, i, err)
    }

    fn display_name() -> String {
        "item end".to_string()
    }

    fn default_error_value() -> Self::Output {
        None
    }
}

struct OptionalType {}
impl CheckedParse for OptionalType {}

impl Parse for OptionalType {
    type Output = Option<SyntaxShape>;

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        let ParseResult {
            value,
            i: i_new,
            err,
        } = IfSuccessThen::<DoublePoint, Shape>::parse(tokens, i);
        if let Some((_, shape)) = value {
            ParseResult::new(Some(shape), i_new, err)
        } else {
            ParseResult::new(None, i, None)
        }
    }

    fn display_name() -> String {
        "type".to_string()
    }

    fn default_error_value() -> Self::Output {
        Some(SyntaxShape::Any)
    }
}

///Returns true if token potentially represents rest argument
fn can_be_rest(token: &Token) -> bool {
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
