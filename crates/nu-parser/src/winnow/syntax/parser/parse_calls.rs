//! Calls: internal, `%` and external calls, their arguments, attributes, and
//! the fixed signatures of the keyword commands parsed as calls (nu-parser's
//! `parse_calls.rs`).

use std::borrow::Cow;

use winnow::Parser;

use crate::winnow::syntax::ast::{
    Argument, Attribute, Call, CallHead, DynamicCall, Expr, Expression, ExternalArgument,
    ExternalCall, InterpolationPart, NamedArgument, Quote, StringInterpolation, StringLiteral,
    SyntaxShape,
};
use crate::winnow::syntax::error::{Diagnostic, ErrorKind};
use crate::winnow::syntax::input::{ParseResult, cut};
use crate::winnow::syntax::lex::{Token, TokenContents};
use crate::winnow::syntax::span::{Span, Spanned};

use super::WorkingSet;
use super::parse_control_flow::{parse_if, parse_match, parse_try};
use super::parse_expressions::{ExpectedShape, parse_value};
use super::parse_helpers::is_spread;
use super::parse_literals::{parse_raw_string, parse_string};
use super::parse_module::parse_import_pattern_member;
use super::parse_source::parse_condition;
use super::tokens::{Tokens, expected, item, repeat_to_end};
use super::working_set::{AliasedKeyword, DeclKind};

/// The most words that can form a known multi-word command.
const MAX_COMMAND_WORDS: usize = 5;

/// A call (nu's `parse_call`): a command name, possibly of several words,
/// followed by arguments.
///
/// ```text
/// call     = "^" external-call | "%" percent-call | head { argument }
/// head     = the longest known command name (`str trim`), else one word
/// argument = "--" | "--" name [ "=" value ] | "-" flags | "..." value | value
/// ```
///
/// A head that resolves to no command is an external command, as in nu: its
/// arguments are external arguments (`git log 0b2d1f4..HEAD`), and so are
/// those of an alias of an external command. Deciding that needs the command
/// table; with none configured every head is a call.
pub fn parse_call<'a>(tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    parse_call_lenient(tokens, false)
}

/// A call whose head is its first word even where a longer command name starts with it: nu
/// picks `run` by its first word wherever it reads a command (`HeadKind::Run`), and `source`,
/// `source-env` and `hide` at the start of a statement (`parse_builtin_commands`).
pub fn parse_first_word_call<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let first = tokens.expect_item("command")?;
    let head = CallHead {
        name: Cow::Borrowed(working_set.get_span_contents(first.span)),
        span: first.span,
    };
    let kind = working_set.find_decl(&head.name);
    parse_call_with_kind(tokens, first.span, head, kind, None, false)
}

/// [`parse_call`]; with `lenient` set (an alias target) a call through an
/// alias of `if`, `match` or `try` is only a call, and `hide`'s members
/// are not checked.
pub fn parse_call_lenient<'a>(
    mut tokens: Tokens<'_, 'a>,
    lenient: bool,
) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let statement = tokens;
    let first = tokens.expect_item("command")?;
    match working_set.get_span_contents(first.span).as_bytes()[0] {
        b'^' => return parse_external_call(first, tokens),
        b'%' => return parse_percent_call(first, tokens),
        _ => {}
    }
    let (head, found) = find_longest_decl(first, &mut tokens, "");
    // What the head is decides how the arguments parse: an alias of an external command or an
    // unknown command takes external arguments, a wrapped command `external_arg` values.
    let kind = match found.or_else(|| working_set.find_decl(&head.name)) {
        Some(DeclKind::ExternalAlias) => {
            let name = StringLiteral {
                value: head.name,
                quote: Quote::Bare,
            };
            let name = Expression::new(Expr::String(name), head.span);
            return parse_external_arguments(None, name, tokens);
        }
        // `alias m = match`: a call to it is that keyword's statement, the alias in the
        // keyword's place (an alias target, `alias m2 = m`, is only a call).
        Some(DeclKind::KeywordAlias(keyword)) if !lenient && head.span == first.span => {
            return match keyword {
                AliasedKeyword::If => parse_if(statement),
                AliasedKeyword::Match => parse_match(statement),
                AliasedKeyword::Try => parse_try(statement),
            };
        }
        None if working_set.has_builtin_decls() => {
            let name = parse_external_string(working_set, head.span)?;
            return parse_external_arguments(None, name, tokens);
        }
        kind => kind,
    };
    parse_call_with_kind(tokens, first.span, head, kind, None, lenient)
}

/// The rest of a call to `head`, a command of `kind`, which starts at `start` (its head, or its
/// `%`): the arguments as nu reads them by the command's signature. A row-condition command's
/// (`any $it > 2`) are one condition, unless the first is a flag, whose value nu takes by the
/// flag's shape, which the parser does not know (`take while --include 2 {..}`): those are
/// read one by one, as any other command's are, and a wrapped command's with the
/// `external_arg` shape.
fn parse_call_with_kind<'a>(
    tokens: Tokens<'_, 'a>,
    start: Span,
    head: CallHead<'a>,
    kind: Option<DeclKind>,
    sigil: Option<Span>,
    lenient: bool,
) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    if kind == Some(DeclKind::OverlayAlias) {
        // The block it is in changes which commands exist (`parse_block`).
        working_set.replace_overlay_alias_called(true);
    }
    let condition = kind == Some(DeclKind::RowCondition)
        && tokens
            .peek_token()
            .is_some_and(|next| !tokens.text(next).starts_with('-'));
    let wrapped = kind == Some(DeclKind::Wrapped);
    let arguments = if condition {
        vec![Argument::Positional(parse_condition(tokens)?)]
    } else {
        parse_call_arguments_with(tokens, wrapped)?
    };
    let span = start.merge(arguments.last().map_or(head.span, Argument::span));
    let call = Call {
        head,
        arguments,
        sigil,
        wrapped,
    };
    check_call(working_set, &call, lenient)?;
    Ok(Expression::new(Expr::Call(call), span))
}

/// The help of every error about a `%` call.
const PERCENT_HELP: &str =
    "write the built-in command's name bare (`%ls`), or `%$var` / `%(expr)` to name it at run time";

/// `%cmd args`, `% cmd args`, `%$var args` and `%(expr) args`: a call that
/// must resolve to a built-in command, never a custom command or alias.
/// `first` is the item starting with `%`, already consumed.
fn parse_percent_call<'a>(first: Token, mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let sigil = Span::new(first.span.start, first.span.start + 1);
    // The head is the rest of the item, or the next item after a bare `%`.
    let head_token = match first.span.len() {
        1 => match tokens.next_token() {
            Some(token) if token.contents == TokenContents::Item => *token,
            _ => {
                return Err(cut(Diagnostic::message(
                    "percent sigil requires a built-in command",
                    sigil,
                )
                .with_help(PERCENT_HELP)));
            }
        },
        _ => Token {
            contents: TokenContents::Item,
            span: Span::new(sigil.end, first.span.end),
        },
    };
    let head_text = working_set.get_span_contents(head_token.span);
    match head_text.as_bytes()[0] {
        b'$' | b'(' => {
            let head = parse_value(working_set, head_token.span, ExpectedShape::Any)?;
            let arguments = parse_call_arguments(tokens)?;
            let span = sigil.merge(arguments.last().map_or(head.span, Argument::span));
            Ok(Expression::new(
                Expr::DynamicCall(DynamicCall {
                    sigil,
                    head: Box::new(head),
                    arguments,
                }),
                span,
            ))
        }
        b'"' | b'\'' | b'`' | b'[' | b'{' | b'^' | b'%' => Err(cut(Diagnostic::message(
            "percent sigil requires a built-in command",
            head_token.span,
        )
        .with_help(PERCENT_HELP))),
        _ => {
            // nu's `find_longest_decl_with_command_type`: the head is the longest name of a
            // built-in command, so `%ls foo` calls `ls` even where `def "ls foo"` exists.
            let builtin = |name: &str| working_set.find_builtin_decl(name);
            let (head, found) = find_longest_name(head_token, &mut tokens, "", builtin);
            let kind = found.or_else(|| working_set.find_builtin_decl(&head.name));
            if kind.is_none() && working_set.has_builtin_decls() {
                return Err(cut(Diagnostic::message(
                    "percent sigil requires a built-in command",
                    head.span,
                )
                .with_help(format!(
                    "`{}` is not a built-in command; {PERCENT_HELP}",
                    head.name
                ))));
            }
            // As nu reads a call to the built-in by its signature: `%any $it > 2` takes a row
            // condition.
            parse_call_with_kind(tokens, sigil, head, kind, Some(sigil), false)
        }
    }
}

/// The longest known command name starting at `first`, already consumed
/// (nu's `find_longest_decl`); the further words of the name are consumed too.
/// `prefix` is `"attr "` for attributes, whose first word carries a leading `@`.
/// Also returns how calls to a name of several words parse, as the lookup that
/// found it answered; `None` when the head is the first word alone, which was
/// not looked up.
pub(super) fn find_longest_decl<'a>(
    first: Token,
    tokens: &mut Tokens<'_, 'a>,
    prefix: &str,
) -> (CallHead<'a>, Option<DeclKind>) {
    let working_set = tokens.working_set;
    find_longest_name(first, tokens, prefix, |name| match prefix {
        "" => working_set.find_decl(name),
        prefix => working_set.find_decl(&format!("{prefix}{name}")),
    })
}

/// [`find_longest_decl`] with the names `known` answers for: how calls to
/// `name` parse, or `None` when it is not a command of the kind looked for.
fn find_longest_name<'a>(
    first: Token,
    tokens: &mut Tokens<'_, 'a>,
    prefix: &str,
    known: impl Fn(&str) -> Option<DeclKind>,
) -> (CallHead<'a>, Option<DeclKind>) {
    let working_set = tokens.working_set;
    let first_word = working_set.get_span_contents(first.span);
    let first_word = if prefix.is_empty() {
        first_word
    } else {
        first_word.strip_prefix('@').unwrap_or(first_word)
    };
    let single = CallHead {
        name: Cow::Borrowed(first_word),
        span: first.span,
    };
    // Fast path: most heads are single words that start no multi-word command.
    let prefix_word = if prefix.is_empty() {
        first_word
    } else {
        prefix.trim_end()
    };
    if !working_set.is_decl_name_prefix(prefix_word) {
        return (single, None);
    }
    // A configured command table has no longer names than `MAX_COMMAND_WORDS`; an engine's
    // commands can have any number of words (`def "a b c d e f" []`), so with one every item
    // may be part of the name, as in nu's `find_longest_decl`.
    let max_words = if working_set.has_lookup() {
        usize::MAX
    } else {
        MAX_COMMAND_WORDS - 1
    };
    // The words after `first` that keep the name within the longest command name; each longer
    // candidate is tried first (nu's `find_longest_decl_with_prefix`).
    let bound = working_set.longest_decl_name();
    let mut length = prefix.len() + first_word.len();
    let remaining = tokens.remaining();
    let fitting = remaining
        .iter()
        .take(max_words)
        .take_while(|token| token.contents == TokenContents::Item)
        .take_while(|token| {
            length += 1 + token.span.len();
            length <= bound
        })
        .count();
    let following = &remaining[..fitting];
    // Most names are written with single spaces between their words: then a candidate is the
    // source text from `first` to its last word, and no name needs to be built.
    let mut end = first.span.end;
    let single_spaced = following
        .iter()
        .take_while(|token| {
            let one_space = token.span.start == end + 1
                && working_set.get_span_contents(Span::new(end, end + 1)) == " ";
            end = token.span.end;
            one_space
        })
        .count();
    for count in (1..=following.len()).rev() {
        let span = first.span.merge(following[count - 1].span);
        let name = if prefix.is_empty() && count <= single_spaced {
            Cow::Borrowed(working_set.get_span_contents(span))
        } else {
            let words = following[..count]
                .iter()
                .map(|token| working_set.get_span_contents(token.span));
            Cow::Owned(
                std::iter::once(first_word)
                    .chain(words)
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        };
        if let Some(kind) = known(&name) {
            for _ in 0..count {
                tokens.next_token();
            }
            return (CallHead { name, span }, Some(kind));
        }
    }
    (single, None)
}

/// `-5` or `-.5`: a negative number rather than a flag.
fn is_negative_number_like(text: &str) -> bool {
    match text.as_bytes() {
        [b'-', digit, ..] if digit.is_ascii_digit() => true,
        [b'-', b'.', digit, ..] => digit.is_ascii_digit(),
        _ => false,
    }
}

/// The arguments of a call: flags, positionals, spreads and `--`.
fn parse_call_arguments<'a>(tokens: Tokens<'_, 'a>) -> ParseResult<Vec<Argument<'a>>> {
    parse_call_arguments_with(tokens, false)
}

/// The arguments of a call; `wrapped` for a [`DeclKind::Wrapped`] command,
/// whose values are parsed with the `external_arg` shape (`f 'x'$`, `f 0b2`).
fn parse_call_arguments_with<'a>(
    mut tokens: Tokens<'_, 'a>,
    wrapped: bool,
) -> ParseResult<Vec<Argument<'a>>> {
    match wrapped {
        true => repeat_to_end(parse_wrapped_call_argument).parse_next(&mut tokens),
        false => repeat_to_end(parse_call_argument).parse_next(&mut tokens),
    }
}

/// One argument of a call. Without the command's signature a flag never
/// takes the next item as its value; only `--name=value` has one.
fn parse_call_argument<'a>(tokens: &mut Tokens<'_, 'a>) -> ParseResult<Argument<'a>> {
    call_argument(tokens, false)
}

/// One argument of a call to a wrapped command.
fn parse_wrapped_call_argument<'a>(tokens: &mut Tokens<'_, 'a>) -> ParseResult<Argument<'a>> {
    call_argument(tokens, true)
}

/// [`parse_call_argument`], its values parsed as external arguments when
/// `wrapped`.
#[inline(always)]
fn call_argument<'a>(tokens: &mut Tokens<'_, 'a>, wrapped: bool) -> ParseResult<Argument<'a>> {
    let token = expected("argument", item).parse_next(tokens)?;
    let working_set = tokens.working_set;
    let text = tokens.text(&token);
    let span = token.span;
    Ok(match text {
        "--" => Argument::EndOfOptions(span),
        _ if text.starts_with("--") && text.len() > 2 => {
            let (name, value_span) = split_long_flag(text, span)?;
            let value = match value_span {
                Some(value_span) => Some(Box::new(parse_argument_value(
                    working_set,
                    value_span,
                    wrapped,
                )?)),
                None => None,
            };
            Argument::Named(NamedArgument {
                span,
                name,
                long: true,
                value,
            })
        }
        _ if text.starts_with('-')
            && text.len() > 1
            && !is_negative_number_like(text)
            && !text.starts_with("-..") =>
        {
            Argument::Named(NamedArgument {
                span,
                name: &text[1..],
                long: false,
                value: None,
            })
        }
        _ if is_spread(text, b"[$({") => {
            let dots = Span::new(span.start, span.start + 3);
            let value_span = Span::new(span.start + 3, span.end);
            Argument::Spread {
                dots,
                expr: parse_value(working_set, value_span, ExpectedShape::Any)?,
            }
        }
        _ => Argument::Positional(parse_argument_value(working_set, span, wrapped)?),
    })
}

/// The name of the long flag `text` at `span` without its dashes, and the span
/// of its value when it is written `--name=value`; `--name=` is an error.
pub(super) fn split_long_flag(text: &str, span: Span) -> ParseResult<(&str, Option<Span>)> {
    let rest = &text[2..];
    match rest.split_once('=') {
        Some((_, "")) => Err(cut(Diagnostic::expected("value after `=`", span.past()))),
        Some((name, _)) => Ok((name, Some(Span::new(span.start + 3 + name.len(), span.end)))),
        None => Ok((rest, None)),
    }
}

/// The value of a switch written `--switch=value`, `--help=value` included:
/// nu parses it as a `bool` (`true`, `false`, or a variable or subexpression
/// it evaluates later), so `--help=1` is an error.
pub(super) fn parse_switch_value<'a>(
    working_set: &WorkingSet<'a>,
    span: Span,
) -> ParseResult<Expression<'a>> {
    parse_value(
        working_set,
        span,
        ExpectedShape::Declared(&SyntaxShape::Boolean),
    )
}

/// The value of an argument: any value, or for a wrapped command an external
/// argument.
fn parse_argument_value<'a>(
    working_set: &WorkingSet<'a>,
    span: Span,
    wrapped: bool,
) -> ParseResult<Expression<'a>> {
    match wrapped {
        true => parse_value(
            working_set,
            span,
            ExpectedShape::Declared(&SyntaxShape::ExternalArgument),
        ),
        false => parse_value(working_set, span, ExpectedShape::Any),
    }
}

/// `^cmd args...` (nu's `parse_external_call`); `first` is the `^cmd` item,
/// already consumed.
fn parse_external_call<'a>(first: Token, tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let caret = Span::new(first.span.start, first.span.start + 1);
    let head_span = Span::new(first.span.start + 1, first.span.end);
    // `^` alone parses in nu (and fails at run time), so the head may be empty.
    let head = match working_set.get_span_contents(head_span).as_bytes().first() {
        Some(b'$' | b'(') => parse_value(working_set, head_span, ExpectedShape::Any)?,
        Some(_) => parse_external_string(working_set, head_span)?,
        None => Expression::new(Expr::String(StringLiteral::bare("")), head_span),
    };
    parse_external_arguments(Some(caret), head, tokens)
}

/// The arguments of an external call to `head`, which starts at `caret`
/// when there is one, and the call.
fn parse_external_arguments<'a>(
    caret: Option<Span>,
    head: Expression<'a>,
    mut tokens: Tokens<'_, 'a>,
) -> ParseResult<Expression<'a>> {
    let arguments = repeat_to_end(parse_external_call_argument).parse_next(&mut tokens)?;
    let start = caret.unwrap_or(head.span);
    let span = start.merge(arguments.last().map_or(head.span, ExternalArgument::span));
    Ok(Expression::new(
        Expr::ExternalCall(ExternalCall {
            caret,
            head: Box::new(head),
            arguments,
        }),
        span,
    ))
}

/// One argument of an external call: a spread `...[a b]`, `...$list`,
/// `...(expr)`, or a regular argument.
fn parse_external_call_argument<'a>(
    tokens: &mut Tokens<'_, 'a>,
) -> ParseResult<ExternalArgument<'a>> {
    let token = expected("argument", item).parse_next(tokens)?;
    let working_set = tokens.working_set;
    if !is_spread(tokens.text(&token), b"[$(") {
        return Ok(ExternalArgument::Regular(parse_external_arg(
            working_set,
            token.span,
        )?));
    }
    let dots = Span::new(token.span.start, token.span.start + 3);
    let value_span = Span::new(token.span.start + 3, token.span.end);
    check_external_list_argument(working_set, value_span)?;
    Ok(ExternalArgument::Spread {
        dots,
        expr: parse_value(working_set, value_span, ExpectedShape::Any)?,
    })
}

/// A regular external argument (nu's `parse_regular_external_arg`): `$vars`,
/// `(...)`, `[...]` and `{...}` are parsed, everything else is an external
/// string.
fn parse_external_arg<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Expression<'a>> {
    match working_set.get_span_contents(span).as_bytes()[0] {
        b'[' => {
            check_external_list_argument(working_set, span)?;
            parse_value(working_set, span, ExpectedShape::Any)
        }
        b'$' | b'(' | b'{' => parse_value(working_set, span, ExpectedShape::Any),
        _ => parse_external_string(working_set, span),
    }
}

/// A `[`-argument of an external command is handed to nu's list parser as
/// it is, which needs the item to END with `]`: `^cmd [a].x` and
/// `^cmd ...[a].x` are "unclosed delimiter", while `[a]` and `(ls).name`
/// parse (`parse_regular_external_arg`, `parse_list_expression`).
fn check_external_list_argument(working_set: &WorkingSet<'_>, span: Span) -> ParseResult<()> {
    let text = working_set.get_span_contents(span);
    if text.starts_with('[') && !text.ends_with(']') {
        let open = Span::new(span.start, span.start + 1);
        return Err(cut(Diagnostic::new(ErrorKind::Unclosed { delimiter: "]", open }, span.past())
            .with_help("an external command's list argument must end with `]`; a cell path after it is not allowed")));
    }
    Ok(())
}

/// The kind of segment [`parse_external_string`] is inside.
enum ExternalStringSegment {
    /// Plain text, up to the next opening quote, backtick or `(`.
    Bare,
    /// A `'...'` or `"..."` string, or a `$'...'` or `$"..."` interpolation;
    /// `escaped` when the byte before escapes this one (a `\` inside double quotes).
    Quote { quote: u8, escaped: bool },
    /// A `` `...` `` string, which has no escapes.
    Backtick,
    /// A `(...)` subexpression, `depth` parentheses deep.
    Paren { depth: usize },
}

/// A word passed to an external command (nu's `parse_external_string`).
///
/// Following Nushell, the word is split into segments (bare text, quoted
/// strings, backtick strings and parenthesised subexpressions) so that
/// `--query='a (b)'` keeps its parentheses literal while `--out=(pwd)/x`
/// interpolates. All-literal words become one string; otherwise the segments
/// form a bare interpolation. The byte loop only finds where the segments
/// start and end; [`parse_string`] parses each.
pub fn parse_external_string<'a>(
    working_set: &WorkingSet<'a>,
    span: Span,
) -> ParseResult<Expression<'a>> {
    let text = working_set.get_span_contents(span);
    let bytes = text.as_bytes();
    if text.starts_with("r#") {
        return parse_raw_string(working_set, span);
    }
    if !bytes
        .iter()
        .any(|byte| matches!(byte, b'"' | b'\'' | b'(' | b')' | b'`'))
    {
        return Ok(Expression::new(
            Expr::String(StringLiteral::bare(text)),
            span,
        ));
    }
    let mut segments: Vec<(usize, usize)> = Vec::new();
    let mut from = 0;
    let mut state = ExternalStringSegment::Bare;
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        match &mut state {
            ExternalStringSegment::Bare => {
                let opener = match byte {
                    b'"' | b'\'' => Some(ExternalStringSegment::Quote {
                        quote: byte,
                        escaped: false,
                    }),
                    b'$' if matches!(bytes.get(index + 1), Some(b'"' | b'\'')) => {
                        Some(ExternalStringSegment::Quote {
                            quote: bytes[index + 1],
                            escaped: false,
                        })
                    }
                    b'`' => Some(ExternalStringSegment::Backtick),
                    b'(' => Some(ExternalStringSegment::Paren { depth: 1 }),
                    _ => None,
                };
                if let Some(next) = opener {
                    if index != from {
                        segments.push((from, index));
                    }
                    from = index;
                    if byte == b'$' {
                        index += 1;
                    }
                    state = next;
                }
            }
            ExternalStringSegment::Quote { quote, escaped } => {
                if byte == *quote && !*escaped {
                    segments.push((from, index + 1));
                    from = index + 1;
                    state = ExternalStringSegment::Bare;
                } else {
                    *escaped = byte == b'\\' && !*escaped && *quote == b'"';
                }
            }
            ExternalStringSegment::Backtick => {
                if byte == b'`' {
                    segments.push((from, index + 1));
                    from = index + 1;
                    state = ExternalStringSegment::Bare;
                }
            }
            ExternalStringSegment::Paren { depth } => match byte {
                b')' if *depth == 1 => {
                    segments.push((from, index + 1));
                    from = index + 1;
                    state = ExternalStringSegment::Bare;
                }
                b')' => *depth -= 1,
                b'(' => *depth += 1,
                _ => {}
            },
        }
        index += 1;
    }
    if from < bytes.len() {
        segments.push((from, bytes.len()));
    }
    let mut parts: Vec<InterpolationPart<'a>> = Vec::with_capacity(segments.len());
    let mut all_text = true;
    for (start, end) in segments {
        let segment = Span::new(span.start + start, span.start + end);
        match parse_string(working_set, segment)?.expr {
            Expr::String(literal) => parts.push(InterpolationPart::Text {
                span: segment,
                value: literal.value,
            }),
            Expr::StringInterpolation(inner) => {
                all_text &= inner
                    .parts
                    .iter()
                    .all(|part| matches!(part, InterpolationPart::Text { .. }));
                parts.extend(inner.parts);
            }
            expr => {
                all_text = false;
                parts.push(InterpolationPart::Expression(Box::new(Expression::new(
                    expr, segment,
                ))));
            }
        }
    }
    let quote = match bytes {
        [b'\'', .., b'\''] => Quote::Single,
        [b'"', .., b'"'] => Quote::Double,
        [b'$', b'"', .., b'"'] if bytes.len() >= 3 => Quote::Double,
        _ => Quote::Bare,
    };
    if all_text {
        let value: Cow<'a, str> = match parts.as_slice() {
            [InterpolationPart::Text { value, .. }] => value.clone(),
            _ => Cow::Owned(
                parts
                    .iter()
                    .filter_map(|part| match part {
                        InterpolationPart::Text { value, .. } => Some(value.as_ref()),
                        InterpolationPart::Expression(_) => None,
                    })
                    .collect(),
            ),
        };
        return Ok(Expression::new(
            Expr::String(StringLiteral { value, quote }),
            span,
        ));
    }
    Ok(Expression::new(
        Expr::StringInterpolation(StringInterpolation { quote, parts }),
        span,
    ))
}

/// `@name arguments` (nu's `parse_attribute`). The name must be non-empty;
/// whether `attr <name>` exists, and the arguments it takes, are the
/// consumer's business (it may come from a `use`d module).
pub fn parse_attribute<'a>(
    working_set: &WorkingSet<'a>,
    attribute_line: &[Token],
) -> ParseResult<Attribute<'a>> {
    let end = attribute_line.last().map_or(0, |token| token.span.end);
    let mut tokens = Tokens::new(working_set, attribute_line, end);
    let first = tokens.expect_item("attribute")?;
    if working_set.get_span_contents(first.span) == "@" {
        return Err(cut(Diagnostic::expected(
            "attribute name after `@`",
            first.span,
        )));
    }
    let (head, _) = find_longest_decl(first, &mut tokens, "attr ");
    let name_span = Span::new(first.span.start + 1, head.span.end);
    let arguments = parse_call_arguments(tokens)?;
    Ok(Attribute {
        span: Span::new(first.span.start, end),
        name: Spanned::new(head.name, name_span),
        arguments,
    })
}

/// The signature nu gives a command that is a keyword there and a call here, as far as the
/// parser checks it. nu's call parser checks the arguments against the whole signature; that
/// is the consumer's, as for any call, but for the structure of a keyword argument.
pub struct KeywordSignature {
    /// A keyword argument (`as NAME`) and how many positionals come before it.
    keyword: Option<(&'static str, usize)>,
}

/// nu's signature for the keyword command `name`, if it is one.
fn keyword_signature(name: &str) -> Option<KeywordSignature> {
    Some(match name {
        "hide" | "source" | "source-env" | "run" | "overlay new" | "overlay hide"
        | "overlay list" | "plugin use" => KeywordSignature { keyword: None },
        "overlay use" => KeywordSignature {
            keyword: Some(("as", 1)),
        },
        _ => return None,
    })
}

/// The fixed signature of `call`, if its head is one of the keyword commands.
/// With no command table configured the head of `overlay use` is `overlay`
/// with `use` as its first argument; both spellings resolve.
pub fn keyword_signature_of_call(call: &Call<'_>) -> Option<KeywordSignature> {
    if let Some(signature) = keyword_signature(&call.head.name) {
        return Some(signature);
    }
    if matches!(&*call.head.name, "overlay" | "plugin")
        && let Some(Argument::Positional(first)) = call.arguments.first()
        && let Expr::String(subcommand) = &first.expr
    {
        return keyword_signature(&format!("{} {}", call.head.name, subcommand.value));
    }
    None
}

/// Check the parts of a call to a keyword command that are syntax in nu, with its arguments read
/// as nu's call parser reads them: a flag before a `--` is no positional, nor is a word like
/// `-1`, which nu takes for a flag, nor a spread, and after the `--` every other argument is
/// one. `hide`'s members, its second positional, are an import pattern, which nu's `parse_hide`
/// hands to `parse_import_pattern` as `use` does (`hide foo null` is a wrong import pattern),
/// unless the call shows help or is an alias's target (`lenient`). `overlay use NAME as ALIAS`
/// has a keyword argument (nu's `SyntaxShape::Keyword`): the positional after the module must
/// be the keyword, and any argument after it is its value (`as --prefix` names the overlay
/// `--prefix`).
pub fn check_call(working_set: &WorkingSet<'_>, call: &Call<'_>, lenient: bool) -> ParseResult<()> {
    let Some(signature) = keyword_signature_of_call(call) else {
        return Ok(());
    };
    let hide = call.head.name == "hide";
    if hide && (lenient || shows_help(call)) || !hide && signature.keyword.is_none() {
        return Ok(());
    }
    let mut arguments = call.arguments.iter();
    // A head resolved as a single word (`overlay` + `use`): skip the subcommand word.
    if keyword_signature(&call.head.name).is_none() {
        arguments.next();
    }
    let mut positionals = 0usize;
    let mut end_of_options = false;
    while let Some(argument) = arguments.next() {
        match argument {
            Argument::EndOfOptions(_) if !end_of_options => {
                end_of_options = true;
                continue;
            }
            Argument::Named(_) if !end_of_options => continue,
            Argument::Positional(expr) if !end_of_options => {
                let text = working_set.get_span_contents(expr.span);
                if text.starts_with('-') && text.len() > 1 {
                    continue;
                }
            }
            // nu fills no positional with a spread here: `hide` refuses one, and for a command
            // with flags `...$x` may be a record of them (`parse_internal_call`).
            Argument::Spread { .. } => continue,
            _ => {}
        }
        positionals += 1;
        if hide {
            if positionals == 2
                && let Argument::Positional(members) = argument
            {
                let token = Token {
                    contents: TokenContents::Item,
                    span: members.span,
                };
                parse_import_pattern_member(working_set, &token, false)?;
            }
            continue;
        }
        let Some((keyword, before)) = signature.keyword else {
            return Ok(());
        };
        if positionals <= before {
            continue;
        }
        let span = argument.span();
        if working_set.get_span_contents(span) != keyword {
            return Err(cut(Diagnostic::new(
                ErrorKind::ExpectedKeyword(keyword),
                span,
            )));
        }
        return match arguments.next() {
            Some(_) => Ok(()),
            None => Err(cut(Diagnostic::message(
                format!("missing argument to `{keyword}`"),
                span.past(),
            ))),
        };
    }
    Ok(())
}

/// Whether `call` has a `--help` or `-h` (alone or in a batch of short flags) before any `--`.
fn shows_help(call: &Call<'_>) -> bool {
    call.arguments
        .iter()
        .take_while(|argument| !matches!(argument, Argument::EndOfOptions(_)))
        .any(|argument| match argument {
            Argument::Named(flag) if flag.long => flag.name == "help",
            Argument::Named(flag) => flag.name.contains('h'),
            _ => false,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::winnow::syntax::ast::Pipeline;
    use crate::winnow::syntax::parser::ParseConfig;

    /// The calls heading the statements of `source`, which must parse.
    fn calls(source: &str) -> Vec<Call<'_>> {
        let (ast, diagnostics) = crate::winnow::syntax::parser::parse(source, &ParseConfig::new());
        assert!(diagnostics.is_empty(), "{source:?}: {diagnostics:?}");
        ast.block
            .pipelines
            .into_iter()
            .filter_map(
                |Pipeline { mut elements, .. }| match elements.remove(0).expr.expr {
                    Expr::Call(call) => Some(call),
                    _ => None,
                },
            )
            .collect()
    }

    #[test]
    fn call_records_whether_its_head_is_wrapped() {
        let calls = calls("def --wrapped w [...rest] {}\nw 'x'$\nls x");
        let wrapped: Vec<(&str, bool)> = calls
            .iter()
            .map(|call| (&*call.head.name, call.wrapped))
            .collect();
        assert_eq!(wrapped, [("w", true), ("ls", false)]);
    }

    #[test]
    fn percent_head_is_the_longest_builtin_name() {
        // nu's `find_longest_decl_with_command_type`: a custom `ls foo` does not hide `ls`.
        let calls = calls("def \"ls foo\" [] {}\n%ls foo\n%str trim");
        let heads: Vec<(&str, usize)> = calls
            .iter()
            .map(|call| (&*call.head.name, call.arguments.len()))
            .collect();
        assert_eq!(heads, [("ls", 1), ("str trim", 0)]);
    }
}
