//! Parser keywords and how nu treats the flags of a keyword command
//! (nu-parser's `parse_keywords.rs`).
//!
//! nu parses every keyword as a call to a command with a fixed signature, so
//! two things hold for all of them and are shared here: at the start of each
//! positional argument nu looks for flags, which for the keywords means that
//! `--help`/`-h` turns the statement into an ordinary call ([`keyword_boundary`])
//! and any other `-x` is an unknown-flag error (`return -1` is one). The
//! commands that are keywords in nu but ordinary calls here (`hide`, `source`,
//! `overlay use`, ...) get their arguments checked by the consumer, as any
//! call's, but for `hide`'s import pattern and the `as` of `overlay use`
//! ([`super::parse_calls::check_call`]).

use crate::winnow::syntax::ast::{Block, Expression};
use crate::winnow::syntax::error::Diagnostic;
use crate::winnow::syntax::input::{ParseResult, cut};
use crate::winnow::syntax::lex::{Token, TokenContents};
use crate::winnow::syntax::span::Span;

use super::WorkingSet;
use super::parse_calls::parse_call;
use super::parse_expressions::{
    ExpectedShape, is_typed_record, parse_block_body, parse_brace_record, parse_value,
};
use super::parse_helpers::{is_help_flag, is_help_text};
use super::tokens::Tokens;

/// Keywords that start a statement and can only appear at the head of a
/// pipeline (they parse their own `=` and `{}` arguments).
pub fn is_statement_keyword(text: &str) -> bool {
    matches!(
        text,
        "def"
            | "extern"
            | "let"
            | "mut"
            | "const"
            | "for"
            | "alias"
            | "type"
            | "module"
            | "use"
            | "export"
            | "export-env"
    )
}

/// nu's `ALIASABLE_PARSER_KEYWORDS`: keywords an alias may name.
pub const ALIASABLE_PARSER_KEYWORDS: &[&str] = &[
    "if",
    "match",
    "try",
    "overlay",
    "overlay hide",
    "overlay new",
    "overlay use",
];

/// nu's `UNALIASABLE_PARSER_KEYWORDS`.
pub const UNALIASABLE_PARSER_KEYWORDS: &[&str] = &[
    "alias",
    "const",
    "def",
    "extern",
    "module",
    "type",
    "use",
    "export",
    "export alias",
    "export const",
    "export def",
    "export extern",
    "export module",
    "export type",
    "export use",
    "for",
    "loop",
    "while",
    "return",
    "break",
    "continue",
    "let",
    "mut",
    "hide",
    "export-env",
    "source-env",
    "source",
    "run",
    "where",
    "plugin use",
];

/// Names that cannot be given to a definition because the parser treats them
/// specially (`nu-parser`'s aliasable and unaliasable keyword tables, the
/// multi-word entries included: `def "export def"` is refused too).
pub fn is_parser_keyword(name: &str) -> bool {
    ALIASABLE_PARSER_KEYWORDS.contains(&name) || UNALIASABLE_PARSER_KEYWORDS.contains(&name)
}

/// What nu does with the item at the start of a keyword's positional argument.
pub enum KeywordBoundary {
    /// `--help`/`-h`: the whole statement is an ordinary call showing help.
    Help,
    /// A flag the keyword does not have (`return -1`, `if --foo`), left in place: nu's call
    /// reports it ([`keyword_flag`]; [`keyword_boundary`] refuses it).
    UnknownFlag,
    /// Nothing special (a `--` end-of-options marker was consumed and ignored).
    Argument,
}

/// Look at the item at a positional boundary of the keyword `keyword`:
/// `--help` and `-h` are its help flag, `--` is consumed and ignored (nu keeps
/// nothing of it), any other `-x` is a flag the keyword does not have
/// (`return -1`), unless `extra` allows it.
pub fn keyword_boundary(
    tokens: &mut Tokens<'_, '_>,
    keyword: &str,
    extra: &[&str],
) -> ParseResult<KeywordBoundary> {
    keyword_boundary_with(tokens, keyword, extra, true)
}

/// [`keyword_boundary`] with a choice about `--`: the statements nu parses by
/// position (`alias`, `module`, `let`, `mut`, `const`, `export-env`) never
/// see the end-of-options marker, so `alias -- x = ls` has no `=` where one
/// is expected.
pub fn keyword_boundary_with(
    tokens: &mut Tokens<'_, '_>,
    keyword: &str,
    extra: &[&str],
    end_of_options: bool,
) -> ParseResult<KeywordBoundary> {
    match keyword_flag(tokens, extra, end_of_options) {
        KeywordBoundary::UnknownFlag => {
            let span = tokens
                .peek_token()
                .map_or(tokens.here(), |token| token.span);
            let text = tokens.working_set.get_span_contents(span);
            Err(cut(Diagnostic::message(
                format!("the `{keyword}` command doesn't have flag `{text}`"),
                span,
            )
            .with_help("use `--help` to see available flags")))
        }
        boundary => Ok(boundary),
    }
}

/// The item at a positional boundary of a keyword, as [`keyword_boundary_with`] reads it, but
/// with a flag the keyword does not have (any `-x` but those `extra` allows) left in place as
/// [`KeywordBoundary::UnknownFlag`], for a statement nu then reads as a call with that flag.
pub fn keyword_flag(
    tokens: &mut Tokens<'_, '_>,
    extra: &[&str],
    end_of_options: bool,
) -> KeywordBoundary {
    let Some(token) = tokens
        .peek_token()
        .filter(|token| token.contents == TokenContents::Item)
    else {
        return KeywordBoundary::Argument;
    };
    if end_of_options && seen_end_of_options(tokens) {
        return KeywordBoundary::Argument;
    }
    let text = tokens.text(token);
    match text {
        _ if is_help_text(text) => KeywordBoundary::Help,
        "--" if end_of_options => {
            tokens.working_set.add_ignored(token.span);
            tokens.next_token();
            KeywordBoundary::Argument
        }
        "--" => KeywordBoundary::Argument,
        _ if text.starts_with('-') && text.len() > 1 && !extra.contains(&text) => {
            KeywordBoundary::UnknownFlag
        }
        _ => KeywordBoundary::Argument,
    }
}

/// Whether a `--` marker was consumed earlier in the statement: from then on
/// nu looks for no flags at all, so a second `--` is a positional (`try {}
/// -- catch {} --` has one too many) and `return -- --help` returns a string.
pub fn seen_end_of_options(tokens: &Tokens<'_, '_>) -> bool {
    tokens.all()[..tokens.position()]
        .iter()
        .any(|token| token.contents == TokenContents::Item && tokens.text(token) == "--")
}

/// A keyword statement parsed the way nu parses a keyword command (nu's
/// `parse_keyword`): at every positional boundary nu looks for flags, so a
/// `--help` (or `-h`) anywhere makes the whole statement an ordinary call
/// showing help. nu still parses every positional after it as usual (`match 1
/// --help :{}` has no match block, `return --help 1 2` has an extra
/// positional) and only forgives the missing ones.
///
/// ```text
/// let mut call = KeywordCall::start(&mut tokens)?;          // `loop`
/// let Some(body) = call.positional(&mut tokens, "block")? else { return call.help_call() };
/// call.end(&mut tokens)?;
/// call.finish(expression)
/// ```
pub struct KeywordCall<'t, 'a> {
    /// The whole statement, parsed again as a call when it asks for help.
    statement: Tokens<'t, 'a>,
    /// The keyword.
    pub keyword: Token,
    /// Whether a `--` ends the options: `false` from
    /// [`KeywordCall::start_positional`], for a keyword whose argument nu takes
    /// by position (`export-env`).
    end_of_options: bool,
    /// A `--help` or `-h` was seen.
    help: bool,
    /// The keyword's call has an error of its own: a flag it does not have (`return --foo`) or
    /// a positional too many (`break x`). nu's call reports it, so the statement is that call,
    /// as for `--help`.
    call_error: bool,
}

impl<'t, 'a> KeywordCall<'t, 'a> {
    /// Consume the keyword at the start of `tokens`.
    pub fn start(tokens: &mut Tokens<'t, 'a>) -> ParseResult<Self> {
        let statement = *tokens;
        let keyword = tokens.expect_item("keyword")?;
        Ok(Self {
            statement,
            keyword,
            end_of_options: true,
            help: false,
            call_error: false,
        })
    }

    /// [`KeywordCall::start`] for a keyword whose argument nu takes by
    /// position, for which `--` is not an end-of-options marker.
    pub fn start_positional(tokens: &mut Tokens<'t, 'a>) -> ParseResult<Self> {
        Ok(Self {
            end_of_options: false,
            ..Self::start(tokens)?
        })
    }

    /// The flags at a positional boundary; nu takes every one of them
    /// (`extern foo --help --help`), a help flag or an unknown one making the statement nu's
    /// call.
    pub fn flags(&mut self, tokens: &mut Tokens<'t, 'a>) -> ParseResult<()> {
        // `keyword_flag` leaves a help flag and an unknown flag for this loop to take and takes a
        // `--` itself; anything else it leaves in place, which ends the flags.
        loop {
            let before = tokens.position();
            match keyword_flag(tokens, &[], self.end_of_options) {
                KeywordBoundary::Help => {
                    tokens.next_token();
                    self.help = true;
                }
                KeywordBoundary::UnknownFlag => {
                    tokens.next_token();
                    self.call_error = true;
                }
                KeywordBoundary::Argument if tokens.position() == before => return Ok(()),
                KeywordBoundary::Argument => {}
            }
        }
    }

    /// Take a help flag at the head of `tokens`, before any `--`; `false` when
    /// there is none. A `def` or an `extern` reads its own flags around it
    /// (`def --help --env f`).
    pub fn help_flag(&mut self, tokens: &mut Tokens<'t, 'a>) -> bool {
        let help = tokens
            .peek_token()
            .is_some_and(|token| is_help_flag(tokens.working_set, token))
            && !(self.end_of_options && seen_end_of_options(tokens));
        if help {
            tokens.next_token();
            self.help = true;
        }
        help
    }

    /// The flags, then the next positional item; `None` when it is missing and the statement is
    /// nu's call ([`Self::is_call`]), which reports it as a call's, or when a flag the keyword
    /// does not have came first ([`Self::call_flags`]).
    pub fn positional(
        &mut self,
        tokens: &mut Tokens<'t, 'a>,
        what: &'static str,
    ) -> ParseResult<Option<Token>> {
        if self.call_flags(tokens)? || self.is_call() && tokens.at_end() {
            return Ok(None);
        }
        tokens.expect_item(what).map(Some)
    }

    /// The flags at a positional boundary ([`Self::flags`]); `true` when a flag the keyword
    /// does not have was seen. nu's first error is then that flag's, whatever its call reads
    /// after it (`if -1 > 0 {}`), since the items before it parsed, so the statement is that
    /// call ([`Self::help_call`]) at once.
    pub fn call_flags(&mut self, tokens: &mut Tokens<'t, 'a>) -> ParseResult<bool> {
        self.flags(tokens)?;
        Ok(self.call_error)
    }

    /// The flags after the last positional, then the end of the statement. An item left is a
    /// positional too many, nu's ExtraPositional (`break x`, `loop {} -- --foo`): the
    /// statement is then nu's call ([`Self::is_call`]).
    pub fn end(&mut self, tokens: &mut Tokens<'t, 'a>) -> ParseResult<()> {
        self.flags(tokens)?;
        if !tokens.at_end() {
            self.call_error = true;
        }
        Ok(())
    }

    /// Whether the statement is nu's ordinary call to the keyword: a `--help` or `-h` was seen,
    /// or the call has an error of its own (a flag the keyword does not have, a positional too
    /// many).
    pub fn is_call(&self) -> bool {
        self.help || self.call_error
    }

    /// The statement as the ordinary call nu makes of it (`keyword --help`, `return --foo`).
    pub fn help_call(self) -> ParseResult<Expression<'a>> {
        parse_help_call(self.statement)
    }

    /// [`Self::help_call`] without `dropped`, an item nu never looks at (the
    /// second brace of `def foo [] {}{} --help`), which is recorded as ignored.
    pub fn help_call_without(self, dropped: &Token) -> ParseResult<Expression<'a>> {
        let working_set = self.statement.working_set;
        let items: Vec<Token> = self
            .statement
            .remaining()
            .iter()
            .filter(|token| token.span != dropped.span)
            .copied()
            .collect();
        let call = parse_help_call(Tokens::new(
            working_set,
            &items,
            self.statement.end_span().start,
        ))?;
        working_set.add_ignored(dropped.span);
        Ok(call)
    }

    /// The finished statement, unless a `--help` or an error of the call made it nu's call.
    pub fn finish(self, expression: Expression<'a>) -> ParseResult<Expression<'a>> {
        if self.is_call() {
            self.help_call()
        } else {
            Ok(expression)
        }
    }
}

/// `keyword --help`: the statement parsed as an ordinary call, as nu does. nu reads the
/// value of a `--help=value` as the switch's `bool` (`if --help=1` is "expected bool",
/// `if --help=false` shows help): that, as the call's other errors, is the consumer's.
pub fn parse_help_call<'a>(statement: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    if let Some(span) = statement.span() {
        statement.working_set.remove_ignored_from(span.start);
    }
    parse_call(statement)
}

/// A `{` item where a keyword wants a block or a closure (`what`) but nu
/// reads a record or a cell path on one ([`parse_brace_record`]). nu then
/// type-checks it: a record typed `record<...>` is neither (`if true {a: 1}`),
/// anything typed `any` passes (`if true { $env.A:b }`, `try {a: 1}.a`).
/// `None` when the item is a block or closure after all.
pub fn parse_brace_value<'a>(
    working_set: &WorkingSet<'a>,
    span: Span,
    what: &'static str,
) -> ParseResult<Option<Expression<'a>>> {
    match parse_brace_record(working_set, span)? {
        Some(value) if is_typed_record(&value) => Err(cut(
            Diagnostic::expected(what, span).with_help("found a record")
        )),
        value => Ok(value),
    }
}

/// The block a keyword command takes (`if`, `while`, `loop`, `try`, `for`),
/// or, as nu-parser's `Block` shape allows, a variable, a subexpression or a
/// brace value ([`parse_brace_value`]) in its place, which nu type-checks as
/// a block (`if $c $env.f`). The block is empty when a value is returned.
pub fn parse_block_or_value_argument<'a>(
    working_set: &WorkingSet<'a>,
    token: &Token,
    what: &'static str,
) -> ParseResult<(Block<'a>, Option<Box<Expression<'a>>>)> {
    match working_set.get_span_contents(token.span).as_bytes().first() {
        Some(b'$' | b'(') if token.contents == TokenContents::Item => {
            let value = parse_value(working_set, token.span, ExpectedShape::Any)?;
            Ok((
                Block {
                    span: token.span,
                    pipelines: Vec::new(),
                },
                Some(Box::new(value)),
            ))
        }
        Some(b'{') if token.contents == TokenContents::Item => {
            match parse_brace_value(working_set, token.span, what)? {
                Some(value) => Ok((
                    Block {
                        span: token.span,
                        pipelines: Vec::new(),
                    },
                    Some(Box::new(value)),
                )),
                None => Ok((parse_block_argument(working_set, token, what)?, None)),
            }
        }
        _ => Ok((parse_block_argument(working_set, token, what)?, None)),
    }
}

/// The `{ ... }` item a statement takes as its block, or an error.
pub fn parse_block_argument<'a>(
    working_set: &WorkingSet<'a>,
    token: &Token,
    what: &'static str,
) -> ParseResult<Block<'a>> {
    if token.contents != TokenContents::Item
        || !working_set.get_span_contents(token.span).starts_with('{')
    {
        return Err(cut(Diagnostic::expected(what, token.span)));
    }
    parse_block_body(working_set, token.span)
}
