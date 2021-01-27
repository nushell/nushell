use log::debug;
use std::marker;

use crate::{lex::Token, parse::util::token_to_spanned_string};
use nu_errors::ParseError;
use nu_source::Span;

use super::ParseResult;

pub(crate) trait Parse {
    type Output;
    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output>;

    fn parse_debug(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        let tokens_str = if i < tokens.len() {
            format!(
                "{:?}",
                &tokens[i..]
                    .iter()
                    .map(|t| t.contents.clone())
                    .collect::<Vec<_>>()
            )
        } else {
            "[]".to_owned()
        };
        debug!(
            r#"Parsing: {:?}
            Tokens: {:?}"#,
            Self::display_name(),
            tokens_str
        );

        Self::parse(tokens, i)
    }

    fn display_name() -> String;
    fn default_error_value() -> Self::Output;

    fn mismatch_error(token: &Token) -> Option<ParseError> {
        Some(ParseError::mismatch(
            Self::display_name(),
            token_to_spanned_string(token),
        ))
    }

    fn mismatch_default_return(token: &Token, i: usize) -> ParseResult<Self::Output> {
        ParseResult::new(Self::default_error_value(), i, Self::mismatch_error(token))
    }
}

pub(crate) trait CheckedParse: Parse {}

pub(crate) struct Expect<Parser: Parse> {
    _marker: marker::PhantomData<*const Parser>,
}

//Expect is always checked
impl<T: Parse> CheckedParse for Expect<T> {}

impl<Parser: Parse> Parse for Expect<Parser> {
    type Output = Parser::Output;

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Parser::Output> {
        if i < tokens.len() {
            debug!(
                "Expect<{:?}> {:?} {:?}",
                Parser::display_name(),
                &tokens[i..],
                i
            );
            //Okay let underlying value parse tokens
            Parser::parse_debug(tokens, i)
        } else {
            debug!("Expect<{:?}> but no tokens", Parser::display_name(),);
            //No tokens are present --> Error out
            let last_span = if let Some(last_token) = tokens.last() {
                last_token.span
            } else {
                Span::unknown()
            };
            ParseResult::new(
                Parser::default_error_value(),
                i,
                Some(ParseError::unexpected_eof(
                    Parser::display_name(),
                    last_span,
                )),
            )
        }
    }

    fn display_name() -> String {
        Parser::display_name()
    }

    fn default_error_value() -> Parser::Output {
        Parser::default_error_value()
    }
}

pub(crate) struct Maybe<Value: CheckedParse> {
    _marker: marker::PhantomData<*const Value>,
}

//Always Checked because accepts only checked
impl<Value: CheckedParse> CheckedParse for Maybe<Value> {}

impl<Value: CheckedParse> Parse for Maybe<Value> {
    type Output = Option<Value::Output>;

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        debug!("Parsing Maybe<{:?}>", Value::display_name());
        let result = Value::parse_debug(tokens, i);
        if result.err.is_some() {
            debug!("Maybe<{:?}> not present", Value::display_name());
            (None, i, None).into()
        } else {
            debug!("Maybe<{:?}> is present", Value::display_name());
            ParseResult::new(Some(result.value), result.i, result.err)
        }
    }

    fn display_name() -> String {
        Value::display_name() + "?"
    }

    fn default_error_value() -> Self::Output {
        Some(Value::default_error_value())
    }
}

///Parse First and (then) Second
pub(crate) struct And2<P1: CheckedParse, P2: CheckedParse> {
    _marker1: marker::PhantomData<*const P1>,
    _marker2: marker::PhantomData<*const P2>,
}

//Always Checked because accepts only checked
impl<P1: CheckedParse, P2: CheckedParse> CheckedParse for And2<P1, P2> {}

impl<P1: CheckedParse, P2: CheckedParse> Parse for And2<P1, P2> {
    type Output = (P1::Output, P2::Output);

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        let _1 = P1::parse(tokens, i);
        let _2 = P2::parse(tokens, _1.i);
        ParseResult::new((_1.value, _2.value), _2.i, _1.err.or(_2.err))
    }

    fn display_name() -> String {
        P1::display_name() + " >> " + &P2::display_name()
    }

    fn default_error_value() -> Self::Output {
        (P1::default_error_value(), P2::default_error_value())
    }
}

pub(crate) struct And3<P1: CheckedParse, P2: CheckedParse, P3: CheckedParse> {
    _marker1: marker::PhantomData<*const P1>,
    _marker2: marker::PhantomData<*const P2>,
    _marker3: marker::PhantomData<*const P3>,
}

//Always Checked because accepts only checked
impl<P1: CheckedParse, P2: CheckedParse, P3: CheckedParse> CheckedParse for And3<P1, P2, P3> {}

impl<P1: CheckedParse, P2: CheckedParse, P3: CheckedParse> Parse for And3<P1, P2, P3> {
    type Output = (P1::Output, P2::Output, P3::Output);

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        let _1 = P1::parse(tokens, i);
        let _2 = P2::parse(tokens, _1.i);
        let _3 = P3::parse(tokens, _2.i);
        ParseResult::new(
            (_1.value, _2.value, _3.value),
            _3.i,
            _1.err.or(_2.err.or(_3.err)),
        )
    }

    fn display_name() -> String {
        P1::display_name() + " >> " + &P2::display_name() + " >> " + &P3::display_name()
    }

    fn default_error_value() -> Self::Output {
        (
            P1::default_error_value(),
            P2::default_error_value(),
            P3::default_error_value(),
        )
    }
}

pub(crate) struct And4<P1: CheckedParse, P2: CheckedParse, P3: CheckedParse, P4: CheckedParse> {
    _marker1: marker::PhantomData<*const P1>,
    _marker2: marker::PhantomData<*const P2>,
    _marker3: marker::PhantomData<*const P3>,
    _marker4: marker::PhantomData<*const P4>,
}

//Always Checked because accepts only checked
impl<P1: CheckedParse, P2: CheckedParse, P3: CheckedParse, P4: CheckedParse> CheckedParse
    for And4<P1, P2, P3, P4>
{
}

impl<P1: CheckedParse, P2: CheckedParse, P3: CheckedParse, P4: CheckedParse> Parse
    for And4<P1, P2, P3, P4>
{
    type Output = (P1::Output, P2::Output, P3::Output, P4::Output);

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        let _1 = P1::parse(tokens, i);
        let _2 = P2::parse(tokens, _1.i);
        let _3 = P3::parse(tokens, _2.i);
        let _4 = P4::parse(tokens, _3.i);
        ParseResult::new(
            (_1.value, _2.value, _3.value, _4.value),
            _4.i,
            _1.err.or(_2.err.or(_3.err.or(_4.err))),
        )
    }

    fn display_name() -> String {
        P1::display_name()
            + " >> "
            + &P2::display_name()
            + " >> "
            + &P3::display_name()
            + " >> "
            + &P4::display_name()
    }

    fn default_error_value() -> Self::Output {
        (
            P1::default_error_value(),
            P2::default_error_value(),
            P3::default_error_value(),
            P4::default_error_value(),
        )
    }
}

pub(crate) struct IfSuccessThen<Try: CheckedParse, AndThen: CheckedParse> {
    _marker1: marker::PhantomData<*const Try>,
    _marker2: marker::PhantomData<*const AndThen>,
}

//Always Checked because accepts only checked
impl<Try: CheckedParse, AndThen: CheckedParse> CheckedParse for IfSuccessThen<Try, AndThen> {}

impl<Try: CheckedParse, AndThen: CheckedParse> Parse for IfSuccessThen<Try, AndThen> {
    type Output = Option<(Try::Output, AndThen::Output)>;

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        let try_result = Maybe::<Try>::parse(tokens, i);
        if let Some(try_v) = try_result.value {
            //Succeeded at parsing Try. Now AndThen has to follow
            let and_then_result = AndThen::parse(tokens, try_result.i);
            ParseResult::new(
                Some((try_v, and_then_result.value)),
                and_then_result.i,
                try_result.err.or(and_then_result.err),
            )
        } else {
            //Okay Couldn't parse Try
            ParseResult::new(None, i, None)
        }
    }

    fn display_name() -> String {
        "(".to_string() + &Try::display_name() + " >> " + &AndThen::display_name() + ")?"
    }

    fn default_error_value() -> Self::Output {
        Some((Try::default_error_value(), AndThen::default_error_value()))
    }
}

pub(crate) struct ParseInto<IntoValue, Parser: CheckedParse> {
    _marker1: marker::PhantomData<*const IntoValue>,
    _marker2: marker::PhantomData<*const Parser>,
}

//Always Checked because accepts only checked
impl<IntoValue: From<Parser::Output>, Parser: CheckedParse> CheckedParse
    for ParseInto<IntoValue, Parser>
{
}

impl<IntoValue: From<Parser::Output>, Parser: CheckedParse> Parse for ParseInto<IntoValue, Parser> {
    type Output = IntoValue;

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        let ParseResult { value, i, err } = Parser::parse(tokens, i);
        let converted: IntoValue = value.into();
        ParseResult::new(converted, i, err)
    }

    fn display_name() -> String {
        Parser::display_name()
    }

    fn default_error_value() -> Self::Output {
        Parser::default_error_value().into()
    }
}

pub(crate) struct WithSpan<Parser: CheckedParse> {
    _marker2: marker::PhantomData<*const Parser>,
}

//Always Checked because accepts only checked
impl<Parser: CheckedParse> CheckedParse for WithSpan<Parser> {}

impl<Parser: CheckedParse> Parse for WithSpan<Parser> {
    type Output = (Span, Parser::Output);

    fn parse(tokens: &[Token], i: usize) -> ParseResult<Self::Output> {
        let i_before = i;
        let ParseResult { value, i, err } = Parser::parse(tokens, i);
        let i_after = i;

        let span = if tokens.len() > 0 {
            //Clamp indices to make sure we never access out of bounds
            let i_before = num_traits::clamp(i_before, 0, tokens.len() - 1);
            let i_after = num_traits::clamp(i_after, 0, tokens.len() - 1);
            tokens[i_before].span.until(tokens[i_after].span)
        } else {
            Span::unknown()
        };

        ParseResult::new((span, value), i, err)
    }

    fn display_name() -> String {
        Parser::display_name()
    }

    fn default_error_value() -> Self::Output {
        (Span::unknown(), Parser::default_error_value())
    }
}
