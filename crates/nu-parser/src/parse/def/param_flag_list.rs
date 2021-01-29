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
    lex::{lex, Token},
    parse::def::lib_code::parse_lib::{And2, CheckedParse, IfSuccessThen, Maybe, Parse},
};
use log::debug;
use nu_errors::ParseError;
use nu_protocol::{NamedType, PositionalType, Signature, SyntaxShape};
use nu_source::{Span, Spanned};

use super::{
    lex_fixup::{lex_split_baseline_tokens_on, lex_split_shortflag_from_longflag},
    lib_code::{
        parse_lib::{And3, OneOf4, Or4, Repeat, Skip, WithSpan},
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

    let (values, _, error, warnings) =
        Repeat::<Or4<EOL, Flag, Rest, Parameter>, Skip>::parse(&tokens, 0).into();
    err = err.or(error);

    for v in values {
        match v {
            OneOf4::V1(_) => {}
            OneOf4::V2(flag) => flags.push(flag),
            OneOf4::V3(r) => rest = Some(r),
            OneOf4::V4(param) => parameters.push(param),
            OneOf4::Err(or_error) => err = err.or(or_error), //Err already given back from Or4
        }
    }

    let signature = to_signature(name, parameters, flags, rest);
    debug!("Signature: {:?}", signature);

    //Caller can't handle warnings yet. Pass them as error for now
    err = err.or_else(|| warnings.first().cloned());

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
            err,
            warnings,
        }
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

        (parameter, i, err, warnings).into()
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
            warnings,
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

        ParseResult::new(flag, i, err, warnings)
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
            value: (_, type_, comment),
            i,
            err,
            warnings,
        } = And3::<RestName, OptionalType, ItemEnd>::parse(tokens, i);

        ParseResult::new(
            (
                type_.unwrap_or(SyntaxShape::Any),
                comment.unwrap_or_else(String::new),
            ),
            i,
            err,
            warnings,
        )
    }

    fn display_name() -> String {
        "Rest item".to_string()
    }

    fn default_error_value() -> Self::Output {
        (SyntaxShape::Any, String::new())
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
            warnings,
        } = And3::<Maybe<Comma>, Maybe<Comment>, Maybe<EOL>>::parse(tokens, i);

        ParseResult::new(comment, i, err, warnings)
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
            warnings,
        } = IfSuccessThen::<DoublePoint, Shape>::parse(tokens, i);
        if let Some((_, shape)) = value {
            ParseResult::new(Some(shape), i_new, err, warnings)
        } else {
            ParseResult::new(None, i, None, vec![])
        }
    }

    fn display_name() -> String {
        "type".to_string()
    }

    fn default_error_value() -> Self::Output {
        Some(SyntaxShape::Any)
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
            .push((param.pos_type, param.desc.unwrap_or_else(String::new)));
    }

    for flag in flags.into_iter() {
        sign.named.insert(
            flag.long_name,
            (flag.named_type, flag.desc.unwrap_or_else(String::new)),
        );
    }

    sign.rest_positional = rest;

    sign
}
