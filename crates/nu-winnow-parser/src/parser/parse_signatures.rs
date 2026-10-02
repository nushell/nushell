//! Signatures (`[a: int, --flag(-f), ...rest]`), variable declarations and
//! input/output type lists (nu-parser's `parse_signatures.rs`).

use std::borrow::Cow;

use winnow::Parser;

use crate::ast::{Comment, InputOutputType, Parameter, ParameterKind, Signature, SyntaxShape, TypeAnnotation};
use crate::error::{Diagnostic, ErrorKind};
use crate::input::{ParseResult, cut};
use crate::lex::{LexOptions, Token, TokenContents, lex};
use crate::span::{Span, Spanned};

use super::WorkingSet;
use super::parse_expressions::{ExpectedShape, parse_value};
use super::parse_helpers::{declared_variable_name, is_identifier, is_variable};
use super::parse_literals::parse_string_literal;
use super::parse_shape_specs::{parse_shape_name, parse_type};
use super::tokens::{Tokens, cut_with, item, keyword, repeat_to_end};

/// A `[...]` or `(...)` signature item (nu's `parse_signature`). `external` is set for an
/// `extern`, whose parameters declare no variables: nu then checks no
/// reserved names and never parses default values.
pub fn parse_signature<'a>(working_set: &WorkingSet<'a>, span: Span, external: bool) -> ParseResult<Signature<'a>> {
    let text = working_set.get_span_contents(span);
    let close = match text.as_bytes().first() {
        Some(b'[') => "]",
        Some(b'(') => ")",
        _ => return Err(cut(Diagnostic::expected("signature", span))),
    };
    if text.len() < 2 || !text.ends_with(close) {
        return Err(cut(Diagnostic::new(
            ErrorKind::Unclosed { delimiter: close, open: Span::new(span.start, span.start + 1) },
            span.past(),
        )));
    }
    parse_signature_helper(working_set, Span::new(span.start + 1, span.end - 1), span, external)
}

/// The parameters in `inner`, the text between the delimiters (nu's
/// `parse_signature_helper`); `outer` becomes the signature's span.
pub fn parse_signature_helper<'a>(
    working_set: &WorkingSet<'a>,
    inner: Span,
    outer: Span,
    external: bool,
) -> ParseResult<Signature<'a>> {
    let tokens = lex(working_set.get_span_contents(inner), inner.start, LexOptions::SIGNATURE).map_err(cut)?;
    let parameters = parse_parameters(working_set, &tokens, external)
        .map_err(|error| error.map(|failure| failure.with_context("signature")))?;
    check_parameter_order(&parameters, outer)?;
    Ok(Signature { span: outer, params: parameters, input_output_types: Vec::new(), input_output_span: None })
}

/// nu's checks over the finished list: a required parameter after an
/// optional one and more than one rest parameter are errors.
fn check_parameter_order(parameters: &[Parameter<'_>], span: Span) -> ParseResult<()> {
    let mut optional_seen = false;
    let mut rest_seen = false;
    for parameter in parameters {
        match parameter.kind {
            ParameterKind::Optional => optional_seen = true,
            ParameterKind::Required if parameter.default.is_some() => optional_seen = true,
            ParameterKind::Required if optional_seen => {
                return Err(cut(Diagnostic::message(
                    format!("required positional parameter `{}` after an optional parameter", parameter.name.item),
                    parameter.span,
                )
                .with_help("move the required parameter before the optional ones")));
            }
            ParameterKind::Rest if rest_seen => {
                return Err(cut(Diagnostic::message("multiple rest params", span)
                    .with_help("a signature can have only one `...rest` parameter")));
            }
            ParameterKind::Rest => rest_seen = true,
            _ => {}
        }
    }
    Ok(())
}

/// What [`parse_parameters`] expects next: nu's `ParseMode` in
/// `parse_signature_helper`, whose rules this follows state by state.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ParseMode {
    /// A parameter, or `:`, `=` or `,` after the one just read.
    Arg,
    /// A parameter after a `,`.
    AfterCommaArg,
    /// The type after a `:`.
    Type,
    /// `=`, `,` or a parameter after a type.
    AfterType,
    /// The default value after a `=`.
    DefaultValue,
}

/// The parameters of a signature, from its tokens lexed with
/// [`LexOptions::SIGNATURE`] (`:`, `=` and `,` are items of their own):
///
/// ```text
/// parameters = { parameter [ "(-" letter ")" ] [ ":" type[@completer] ] [ "=" default ] [ "," ] }
/// ```
///
/// A comment after a parameter is its description.
fn parse_parameters<'a>(
    working_set: &WorkingSet<'a>,
    tokens: &[Token],
    external: bool,
) -> ParseResult<Vec<Parameter<'a>>> {
    let mut parameters: Vec<Parameter<'a>> = Vec::new();
    let mut mode = ParseMode::Arg;
    let items: Vec<&Token> = tokens.iter().filter(|token| token.contents != TokenContents::Eof).collect();
    for (index, token) in items.iter().enumerate() {
        let last = index + 1 == items.len();
        let text = working_set.get_span_contents(token.span);
        match token.contents {
            TokenContents::Comment => {
                working_set.add_comment(token.span);
                if let Some(parameter) = parameters.last_mut() {
                    parameter.description.push(Comment { span: token.span });
                }
                continue;
            }
            // nu skips every token that is not an item: pipes, `;`, redirections.
            TokenContents::Pipe
            | TokenContents::PipePipe
            | TokenContents::Semicolon
            | TokenContents::Redirection(_) => continue,
            TokenContents::Item | TokenContents::AssignmentOperator(_) => {}
            TokenContents::Eol | TokenContents::Eof => continue,
        }
        match text {
            ":" => match mode {
                ParseMode::Arg if last => return Err(cut(Diagnostic::expected("type", token.span.past()))),
                ParseMode::Arg => mode = ParseMode::Type,
                ParseMode::AfterCommaArg | ParseMode::AfterType => {
                    return Err(cut(Diagnostic::expected("parameter or flag", token.span)));
                }
                ParseMode::Type | ParseMode::DefaultValue => return Err(cut(Diagnostic::expected("type", token.span))),
            },
            "=" => match mode {
                ParseMode::Arg | ParseMode::AfterType if last => {
                    return Err(cut(Diagnostic::expected("default value", token.span.past())));
                }
                ParseMode::Arg | ParseMode::AfterType => mode = ParseMode::DefaultValue,
                ParseMode::Type => return Err(cut(Diagnostic::expected("type", token.span))),
                ParseMode::AfterCommaArg => return Err(cut(Diagnostic::expected("parameter or flag", token.span))),
                ParseMode::DefaultValue => return Err(cut(Diagnostic::expected("default value", token.span))),
            },
            "," => match mode {
                ParseMode::Arg | ParseMode::AfterType => mode = ParseMode::AfterCommaArg,
                ParseMode::AfterCommaArg => return Err(cut(Diagnostic::expected("parameter or flag", token.span))),
                ParseMode::Type => return Err(cut(Diagnostic::expected("type", token.span))),
                ParseMode::DefaultValue => return Err(cut(Diagnostic::expected("default value", token.span))),
            },
            _ => match mode {
                ParseMode::Arg | ParseMode::AfterCommaArg | ParseMode::AfterType => {
                    if let Some(short) = text.strip_prefix("(-") {
                        // `--long (-s)`: a short alias for the preceding flag.
                        if mode == ParseMode::AfterCommaArg {
                            return Err(cut(Diagnostic::expected("parameter or flag", token.span)));
                        }
                        let short = short
                            .strip_suffix(')')
                            .ok_or_else(|| cut(Diagnostic::expected("short flag like `(-s)`", token.span)))?;
                        let mut chars = short.chars();
                        let (Some(letter), None) = (chars.next(), chars.next()) else {
                            return Err(cut(Diagnostic::expected("single-character short flag", token.span)));
                        };
                        match parameters.last_mut() {
                            Some(Parameter { kind: ParameterKind::Flag { short: slot @ None, .. }, span, .. }) => {
                                *slot = Some(Spanned::new(letter, Span::new(token.span.start + 2, token.span.end - 1)));
                                *span = span.merge(token.span);
                            }
                            Some(Parameter { kind: ParameterKind::Flag { short: Some(_), .. }, .. }) => {
                                return Err(cut(Diagnostic::message("this flag already has a short form", token.span)));
                            }
                            _ => {
                                return Err(cut(Diagnostic::message(
                                    "short flag alias without a preceding long flag",
                                    token.span,
                                )));
                            }
                        }
                        continue;
                    }
                    parameters.push(parse_parameter(working_set, token, external)?);
                    mode = ParseMode::Arg;
                }
                ParseMode::Type => {
                    // `[: int]`: nu silently drops a type with no parameter before it.
                    if parameters.is_empty() {
                        mode = ParseMode::AfterType;
                        continue;
                    }
                    let (ty, completer) = parse_shape_name(working_set, token.span)?;
                    let Some(parameter) = parameters.last_mut() else { unreachable!("checked above") };
                    if let ParameterKind::Flag { .. } = parameter.kind
                        && ty.shape == SyntaxShape::Boolean
                    {
                        return Err(cut(Diagnostic::message(
                            "type annotations are not allowed for boolean switches",
                            token.span,
                        )
                        .with_help("remove the `: bool` type annotation")));
                    }
                    parameter.ty = Some(ty);
                    parameter.completer = completer;
                    parameter.span = parameter.span.merge(token.span);
                    mode = ParseMode::AfterType;
                }
                ParseMode::DefaultValue => {
                    // `[= 1]`: nu silently drops a default with no parameter before it.
                    if parameters.is_empty() {
                        mode = ParseMode::Arg;
                        continue;
                    }
                    if external {
                        // nu never parses the default values of an `extern` signature.
                        working_set.add_ignored(token.span);
                        mode = ParseMode::Arg;
                        continue;
                    }
                    let Some(parameter) = parameters.last_mut() else { unreachable!("checked above") };
                    if let ParameterKind::Rest = parameter.kind {
                        return Err(cut(Diagnostic::message("rest parameter was given a default value", token.span)
                            .with_help("a `...rest` parameter can't have a default value")));
                    }
                    // The default is parsed with the declared shape (`[x: int = abc]` is an error).
                    let default = match &parameter.ty {
                        Some(declared) => {
                            parse_value(working_set, token.span, ExpectedShape::Declared(&declared.shape))?
                        }
                        None => parse_value(working_set, token.span, ExpectedShape::Any)?,
                    };
                    parameter.default = Some(default);
                    parameter.span = parameter.span.merge(token.span);
                    mode = ParseMode::Arg;
                }
            },
        }
    }
    // Like nu, a `:` or `=` that is not the last token (a comment may follow
    // it) leaves the list as it is: `[x: # c\n]` is a parameter without a type.
    Ok(parameters)
}

/// One parameter item:
///
/// ```text
/// parameter = "--" long [ "(-" letter ")" ]   a flag with a long name
///           | "-" letter                      a flag with only a short name
///           | "..." name                      the rest parameter
///           | name "?"                        an optional positional
///           | name                            a required positional
/// ```
///
/// `external` is set for an `extern`, whose parameters declare no variables.
fn parse_parameter<'a>(working_set: &WorkingSet<'a>, token: &Token, external: bool) -> ParseResult<Parameter<'a>> {
    let text = working_set.get_span_contents(token.span);
    let span = token.span;
    let base = Parameter {
        span,
        kind: ParameterKind::Required,
        name: Spanned::new("", span),
        ty: None,
        default: None,
        completer: None,
        description: Vec::new(),
    };
    // A parameter declares a variable, whose name may not be a reserved one
    // (`in`, `nu`, `env`, `ans`); an extern's parameters declare nothing.
    let declare =
        |name: &str, span: Span| if external { Ok(()) } else { ensure_not_reserved_variable_name(name, span) };
    if let Some(rest) = text.strip_prefix("--").filter(|rest| !rest.is_empty()) {
        // `--long` or `--long(-s)`
        let (long, short) = match rest.split_once('(') {
            None => (rest, None),
            Some((long, short)) => {
                let short = short.strip_prefix('-').and_then(|short| short.strip_suffix(')'));
                let Some(short) = short else {
                    return Err(cut(Diagnostic::expected("short flag alternative like `--flag(-f)`", span)));
                };
                let mut chars = short.chars();
                let (Some(letter), None) = (chars.next(), chars.next()) else {
                    return Err(cut(Diagnostic::expected("single-character short flag", span)));
                };
                let short_start = span.start + 2 + long.len() + 2;
                (long, Some(Spanned::new(letter, Span::new(short_start, short_start + letter.len_utf8()))))
            }
        };
        let variable = long.replace('-', "_");
        if !is_identifier(&variable) {
            return Err(cut(Diagnostic::expected("valid name for this long flag", span)));
        }
        let long_span = Span::new(span.start + 2, span.start + 2 + long.len());
        declare(&variable, long_span)?;
        return Ok(Parameter {
            kind: ParameterKind::Flag { long: Some(Spanned::new(long, long_span)), short },
            name: Spanned::new(long, long_span),
            ..base
        });
    }
    if let Some(short) = text.strip_prefix('-').filter(|short| !short.is_empty()) {
        let mut chars = short.chars();
        let (Some(letter), None) = (chars.next(), chars.next()) else {
            return Err(cut(Diagnostic::expected("single-character short flag", span)));
        };
        // `-.` and `--`: the letter must be an identifier byte.
        if !is_identifier(short) {
            return Err(cut(Diagnostic::expected("valid variable name for this short flag", span)));
        }
        let short_span = Span::new(span.start + 1, span.end);
        return Ok(Parameter {
            kind: ParameterKind::Flag { long: None, short: Some(Spanned::new(letter, short_span)) },
            name: Spanned::new(short, short_span),
            ..base
        });
    }
    if let Some(name) = text.strip_suffix('?') {
        if !is_identifier(name) {
            return Err(cut(Diagnostic::expected("valid variable name for this optional parameter", span)));
        }
        let name_span = Span::new(span.start, span.end - 1);
        declare(name, name_span)?;
        return Ok(Parameter { kind: ParameterKind::Optional, name: Spanned::new(name, name_span), ..base });
    }
    if let Some(name) = text.strip_prefix("...") {
        if !is_identifier(name) {
            return Err(cut(Diagnostic::expected("valid variable name for this rest parameter", span)));
        }
        let name_span = Span::new(span.start + 3, span.end);
        declare(name, name_span)?;
        return Ok(Parameter { kind: ParameterKind::Rest, name: Spanned::new(name, name_span), ..base });
    }
    if !is_identifier(text) {
        return Err(cut(Diagnostic::expected("valid variable name for this parameter", span)));
    }
    declare(text, span)?;
    Ok(Parameter { name: Spanned::new(text, span), ..base })
}

/// nu's `parse_full_signature`: the items nu hands to the signature argument
/// of `def`/`extern` are everything up to the body. One item is the
/// signature; two of which the second starts with `{` is the signature and
/// an item nu drops on the floor; otherwise the input/output types follow a
/// `:` (attached to the signature or standing alone), possibly none.
pub fn parse_full_signature<'a>(
    working_set: &WorkingSet<'a>,
    items: &[Token],
    external: bool,
) -> ParseResult<Signature<'a>> {
    let signature_item = |token: &Token| -> ParseResult<Span> {
        let text = working_set.get_span_contents(token.span);
        match token.contents == TokenContents::Item && text.starts_with(['[', '(']) {
            true => Ok(token.span),
            false => Err(cut(Diagnostic::expected("signature like `[param: type]`", token.span))),
        }
    };
    let (first, rest) = match items {
        [] => return Err(cut(Diagnostic::expected("signature", Span::point(0)))),
        [only] => return parse_signature(working_set, signature_item(only)?, external),
        [first, second] if working_set.get_span_contents(second.span).starts_with('{') => {
            working_set.add_ignored(second.span);
            return parse_signature(working_set, signature_item(first)?, external);
        }
        [first, rest @ ..] => (first, rest),
    };
    let signature_span = signature_item(first)?;
    let (signature_span, type_items) = match working_set.get_span_contents(first.span).strip_suffix(':') {
        Some(_) => (Span::new(signature_span.start, signature_span.end - 1), rest),
        None if working_set.get_span_contents(rest[0].span) == ":" => (signature_span, &rest[1..]),
        None => return Err(cut(Diagnostic::expected("`:` before the input/output types", rest[0].span))),
    };
    let mut signature = parse_signature(working_set, signature_span, external)?;
    if let (Some(first), Some(last)) = (type_items.first(), type_items.last()) {
        let span = first.span.merge(last.span);
        signature.input_output_types = parse_input_output_types(working_set, span)?;
        signature.input_output_span = Some(span);
        signature.span = signature.span.merge(span);
    }
    Ok(signature)
}

/// The input/output types covering `span` (nu's `parse_input_output_types`):
/// one `input -> output` pair, or a bracketed list of them.
///
/// ```text
/// input-output-types = pair | "[" { pair [ "," ] } "]"
/// pair               = type "->" type
/// ```
fn parse_input_output_types<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Vec<InputOutputType<'a>>> {
    // Like nu, a leading `[` and a trailing `]` are dropped independently, so
    // `nothing -> nothing]` is one pair.
    let text = working_set.get_span_contents(span);
    let start = span.start + usize::from(text.starts_with('['));
    let end = (span.end - usize::from(text.ends_with(']'))).max(start);
    let inner = Span::new(start, end);
    let lexed = lex(working_set.get_span_contents(inner), inner.start, LexOptions::IO_TYPES).map_err(cut)?;
    // Like nu, every token counts by position, so a `;` or `|` is read as a
    // type or an arrow and refused (`[int ; -> int]`).
    let items: Vec<Token> = lexed
        .into_iter()
        .filter(|token| !matches!(token.contents, TokenContents::Eof | TokenContents::Eol | TokenContents::Comment))
        .map(|token| Token { contents: TokenContents::Item, span: token.span })
        .collect();
    repeat_to_end(input_output_type).parse_next(&mut Tokens::new(working_set, &items, inner.end))
}

/// One `input -> output` pair.
fn input_output_type<'a>(tokens: &mut Tokens<'_, 'a>) -> ParseResult<InputOutputType<'a>> {
    let working_set = tokens.working_set;
    let input = item(tokens)?;
    let input_type = parse_type(working_set, input.span)?;
    let arrow = cut_with(keyword("->"), |tokens| {
        Diagnostic::expected("arrow (->)", tokens.peek_token().map_or(input.span.past(), |token| token.span))
    })
    .parse_next(tokens)?;
    let output = cut_with(item, |_| Diagnostic::expected("output type", arrow.span.past())).parse_next(tokens)?;
    Ok(InputOutputType { input: input_type, arrow: arrow.span, output: parse_type(working_set, output.span)? })
}

/// The quoted or bare name of a definition (`def "foo bar"`), without quotes.
pub fn parse_definition_name<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Spanned<Cow<'a, str>>> {
    let literal = parse_string_literal(working_set, span)?;
    Ok(Spanned::new(literal.value, span))
}

/// A declared variable name: `x`, `$x`, or `x:` (followed by a type).
/// Returns the name and whether a type follows.
pub fn parse_var_with_opt_type<'a>(
    working_set: &WorkingSet<'a>,
    token: &Token,
) -> ParseResult<(Spanned<&'a str>, bool)> {
    let text = working_set.get_span_contents(token.span);
    let (text, typed) = match text.strip_suffix(':') {
        Some(text) => (text, true),
        None => (text, false),
    };
    if text.contains([' ', '"', '\'', '`']) || !is_variable(text) {
        return Err(cut(Diagnostic::expected("valid variable name", token.span)
            .with_help("variable names may not contain spaces, quotes or `.[({+-*^%/=!<>&|`")));
    }
    let (offset, name) = declared_variable_name(text);
    let start = token.span.start + offset;
    let span = Span::new(start, start + name.len());
    ensure_not_reserved_variable_name(name, span)?;
    Ok((Spanned::new(name, span), typed))
}

/// The type annotation items between a declared name and `=`: `x: int`,
/// `x : int`, `x: record<a: int, b: string>` (several items, re-lexed as one).
pub fn parse_type_after_var<'a>(
    working_set: &WorkingSet<'a>,
    items: &[Token],
    typed: bool,
    after_name: Span,
) -> ParseResult<Option<TypeAnnotation<'a>>> {
    // `let a : int`: like Nushell, the colon must be attached to the name.
    if let Some(first) = items.first()
        && working_set.get_span_contents(first.span) == ":"
    {
        return Err(cut(Diagnostic::new(ErrorKind::ExtraTokens, first.span)));
    }
    match (typed, items.first(), items.last()) {
        (true, Some(first), Some(last)) => Ok(Some(parse_var_type(working_set, first.span.merge(last.span))?)),
        (true, ..) => Err(cut(Diagnostic::expected("type after `:`", after_name))),
        (false, Some(first), _) => Err(cut(Diagnostic::new(ErrorKind::ExtraTokens, first.span))),
        (false, ..) => Ok(None),
    }
}

/// The type of a declared variable (`let x: T`, `for x: T in`), which nu
/// lexes as a signature before it reads the type, so that `record<a: int,
/// b]>` is an unbalanced delimiter.
pub fn parse_var_type<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<TypeAnnotation<'a>> {
    lex(working_set.get_span_contents(span), span.start, LexOptions::VAR_TYPE).map_err(cut)?;
    parse_type(working_set, span)
}

/// Variable names nu reserves (`NameIsBuiltinVar`).
fn is_reserved_variable(name: &str) -> bool {
    matches!(name, "in" | "nu" | "env" | "ans")
}

/// Refuse a reserved variable name in a declaration.
pub fn ensure_not_reserved_variable_name(name: &str, span: Span) -> ParseResult<()> {
    match is_reserved_variable(name) {
        true => Err(cut(Diagnostic::message(format!("`{name}` used as variable name"), span)
            .with_help(format!("`${name}` is a built-in variable and cannot be declared")))),
        false => Ok(()),
    }
}
