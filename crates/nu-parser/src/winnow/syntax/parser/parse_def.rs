//! `def`, `extern`, `for`, attribute blocks and the predeclaration of
//! commands (nu-parser's `parse_def.rs`).

use std::borrow::Cow;

use winnow::Parser;
use winnow::combinator::opt;
use winnow::error::ErrMode;

use crate::winnow::syntax::ast::{
    Block, Def, DefFlag, DefinitionFlag, Expr, Expression, Extern, ExternFlag, For, Signature,
};
use crate::winnow::syntax::error::{Diagnostic, ErrorKind};
use crate::winnow::syntax::input::{ParseFailure, ParseResult, cut};
use crate::winnow::syntax::lex::{LexOptions, Token, TokenContents};
use crate::winnow::syntax::span::{Span, Spanned};

use super::WorkingSet;
use super::lite_parser::{AfterPipe, after_pipe_lines, pipe_on_later_line};
use super::parse_calls::{parse_definition_call, parse_flag_value, split_long_flag};
use super::parse_expressions::{
    BraceShape, ExpectedShape, brace_shape, parse_block_body_unchecked, parse_closure_parts,
    parse_value,
};
use super::parse_helpers::{is_help_flag, is_help_text, is_spread};
use super::parse_keywords::{
    KeywordCall, call_after_error, is_parser_keyword, parse_block_or_value_argument,
    parse_help_call, seen_end_of_options,
};
use super::parse_literals::is_bare_string_interpolation;
use super::parse_signatures::{
    parse_definition_name, parse_full_signature, parse_var_type, parse_var_with_opt_type,
};
use super::tokens::{Tokens, item, tokens_until};
use super::working_set::{CommandLookup, DeclKind};

/// Declare the names of the `def`/`extern` statements of a block before
/// parsing it (nu's `parse_def_predecl`, which nu's `parse_block` calls first
/// for every pipeline of one command), so calls to commands defined later
/// resolve. Like nu, only a definition with a signature item after its name is
/// predeclared, and a name declared twice in one block is an error. An
/// `alias` is declared when its statement is parsed (`WorkingSet::add_alias`),
/// so a call before it is an unknown command.
pub fn parse_def_predecl(working_set: &WorkingSet<'_>, tokens: &[Token]) {
    scan_definitions(working_set, tokens, |_| {}, |_| {});
}

/// A `type` statement of a block found by the predeclaration scan (nu's `parse_def_predecl`
/// records it with `add_type_predecl`): only the name and where the statement is, so that an
/// engine can tell a use of the alias above its definition, or inside it, from an unknown type.
/// The type itself is read when the statement is parsed, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeAliasPredecl {
    /// The name item, as written.
    pub name: Span,
    /// The statement's items, from `type` (or the `export` before it) to its last item.
    pub span: Span,
}

/// A `def` or `extern` of a block found by the predeclaration scan, with its
/// signature parsed: what an engine needs to declare the command before the
/// block's statements are parsed (see [`crate::winnow::syntax::BlockSink::predecl`]). Every
/// definition with a name item is reported, as nu's predeclaration checks the
/// name of each; only those with a signature are declared.
#[derive(Clone, Debug, PartialEq)]
pub struct PredeclaredDef<'a> {
    /// The statement's items, from `def`/`extern` (or the `export` before it)
    /// to its last item.
    pub span: Span,
    /// The command's name, unquoted.
    pub name: Spanned<Cow<'a, str>>,
    /// The signature, with its input/output types; `None` when there is none
    /// or the name or the signature does not parse.
    pub signature: Option<Signature<'a>>,
    /// A `def --wrapped`, whose unknown flags and arguments are strings (and whose untyped
    /// rest parameter takes external arguments).
    pub wrapped: bool,
    /// An `extern` rather than a `def`.
    pub external: bool,
}

/// [`parse_def_predecl`], also returning the definitions found, whose signatures
/// [`Definitions::parse`] parses later: the statements need only the declared
/// names, so they can be parsed meanwhile (see `BlockStatements::new`).
pub(super) fn scan_predecls(working_set: &WorkingSet<'_>, tokens: &[Token]) -> Definitions {
    let mut definitions = Definitions::default();
    scan_definitions(
        working_set,
        tokens,
        |definition| {
            // Copied, so that `Definitions` borrows nothing.
            let items = definition.items.to_vec();
            definitions.defs.push(definition.with_items(items));
        },
        |type_alias| definitions.type_aliases.push(type_alias),
    );
    definitions
}

/// The `def`/`extern` statements of a block, as the predeclaration scan found
/// them (see `BlockStatements::new`), their signatures not yet
/// parsed, and its `type` statements.
#[derive(Debug, Default)]
pub struct Definitions {
    /// The `def`/`extern` statements.
    defs: Vec<FoundDefinition<Vec<Token>>>,
    /// The `type` statements, in source order.
    type_aliases: Vec<TypeAliasPredecl>,
}

impl Definitions {
    /// The block's `type` statements, in source order, which an engine records before it
    /// takes any statement, as it declares the definitions.
    pub fn type_aliases(&self) -> &[TypeAliasPredecl] {
        &self.type_aliases
    }

    /// Each definition with its signature, parsed in `source` with the
    /// commands `lookup` knows. A signature is parsed as the scan would have
    /// parsed it: with the names of the block's definitions up to and
    /// including its own declared. What parsing a name or signature reports is
    /// dropped, as nu's predeclaration drops it (the statement reports it
    /// again when parsed).
    pub fn parse<'a>(
        self,
        source: &'a str,
        span: Span,
        lookup: impl CommandLookup + 'a,
    ) -> Vec<PredeclaredDef<'a>> {
        let working_set = WorkingSet::with_lookup(source, span, lookup);
        self.defs
            .into_iter()
            .map(|found| {
                // A new working set: the scan's names are declared again, in its order.
                if let Some(kind) = found.declared
                    && let Ok(name) = parse_definition_name(&working_set, found.name.span)
                {
                    working_set.add_predecl(&name.item, kind);
                }
                let def = predeclared_def(&working_set, found.with_items(found.items.as_slice()));
                drop(working_set.take_errors_from(0));
                def
            })
            .collect()
    }
}

/// A definition found by [`scan_definitions`], with its items (`&[Token]` while
/// scanning, `Vec<Token>` in [`Definitions`]).
#[derive(Debug)]
struct FoundDefinition<Items> {
    /// The statement's items, from its first to its last.
    span: Span,
    /// The items of the statement after `def`/`extern`: flags, name, signature, body.
    items: Items,
    /// The name item.
    name: Token,
    /// Whether an item starting with `[` or `(` follows the name: only then is
    /// the name declared.
    has_signature: bool,
    /// Whether the statement has the `--wrapped` flag (looked for only when
    /// the name is declared).
    wrapped: bool,
    /// An `extern` rather than a `def`.
    external: bool,
    /// How the scan declared the name; `None` when it did not.
    declared: Option<DeclKind>,
}

impl<Items> FoundDefinition<Items> {
    /// This definition with `items` for its items.
    fn with_items<Other>(&self, items: Other) -> FoundDefinition<Other> {
        let FoundDefinition {
            span,
            name,
            has_signature,
            wrapped,
            external,
            declared,
            ..
        } = *self;
        FoundDefinition {
            span,
            items,
            name,
            has_signature,
            wrapped,
            external,
            declared,
        }
    }
}

/// A definition found by the scan, with its signature: the items from the
/// first one starting with `[` or `(` after the name, up to (not including) a
/// `def`'s body. A name that is no string (`def "a"b`, or a bare interpolation
/// such as `def $"a(1)"`, which nu's `parse_string` reads as one) gets no
/// signature, so the engine predeclares nothing, as nu does.
fn predeclared_def<'a>(
    working_set: &WorkingSet<'a>,
    found: FoundDefinition<&[Token]>,
) -> PredeclaredDef<'a> {
    let text = working_set.get_span_contents(found.name.span);
    let name = match is_bare_string_interpolation(text) {
        true => None,
        false => parse_definition_name(working_set, found.name.span).ok(),
    };
    let (name, has_signature) = match name {
        Some(name) => (name, found.has_signature),
        None => (Spanned::new(Cow::Borrowed(text), found.name.span), false),
    };
    let signature = has_signature.then(|| {
        let after_name = found
            .items
            .iter()
            .position(|token| token.span == found.name.span)?
            + 1;
        let signature_start = after_name
            + found.items[after_name..].iter().position(|token| {
                working_set
                    .get_span_contents(token.span)
                    .starts_with(['[', '('])
            })?;
        // A `def`'s last item is its body unless it is the only one (`def f []`); every
        // item of an `extern` is its signature's.
        let signature_items = match &found.items[signature_start..] {
            [only] => std::slice::from_ref(only),
            [signature @ .., _body] if !found.external => signature,
            all => all,
        };
        parse_full_signature(working_set, signature_items, found.external).ok()
    });
    let signature = signature.flatten();
    PredeclaredDef {
        span: found.span,
        name,
        signature,
        wrapped: found.wrapped,
        external: found.external,
    }
}

/// The predeclaration scan: declare every `def`/`extern` of a block that has a
/// signature item after its name, report a name declared twice, and hand each
/// definition with a name (a statement of at least three items, as nu
/// requires) to `on_definition`, and each `type` statement with a name and an
/// `=` after it to `on_type_alias`.
///
/// It runs before the lite parse, over the block's tokens, so it finds where
/// statements start itself: at an item first on its line or after a `;`, past
/// a `|` that leads a new statement but not one that continues a pipeline.
/// Whether a pipe joins a command to its pipeline is the statement parser's
/// rule ([`after_pipe_lines`]). Of each statement it reads only the head and
/// the tokens up to its end of line.
fn scan_definitions<'a>(
    working_set: &WorkingSet<'a>,
    tokens: &[Token],
    mut on_definition: impl FnMut(FoundDefinition<&[Token]>),
    mut on_type_alias: impl FnMut(TypeAliasPredecl),
) {
    let mut declared: Vec<Cow<'a, str>> = Vec::new();
    // A `|`, or a redirection into one (`e>|`).
    let is_pipe = |token: &Token| match token.contents {
        TokenContents::Pipe => true,
        TokenContents::Redirection(operator) => operator.is_pipe(),
        _ => false,
    };
    // Whether a command follows the pipe at `tokens[at]` in its pipeline, as the statement
    // parser decides it; of several pipes in a row (`a | | b`, `a |\n| b`) the last decides.
    let joins_next_command = |mut at: usize| loop {
        let mut after = Tokens::new(working_set, &tokens[at + 1..], 0);
        match after_pipe_lines(&mut after, tokens[at].contents == TokenContents::Pipe) {
            Ok((_, AfterPipe::Command)) => match after.peek_token() {
                Some(next) if next.contents == TokenContents::Pipe => at += 1 + after.position(),
                _ => break true,
            },
            _ => break false,
        }
    };
    // The next item is the first of its line, or the first after a `;`.
    let mut at_line_start = true;
    // No command since the start of the block, a `;` or a blank line: a `|` here leads the next
    // command (`|def x [] {}`) instead of joining a pipeline.
    let mut after_statement = true;
    // The next item continues a pipeline, even first on its line: the last pipe has a command
    // after it (`ls |\ndef f [] {}`, `ls |\n\n| def f [] {}`), or an attribute line in a
    // pipeline came last, whose command follows on the next line.
    let mut after_pipe = false;
    // On an attribute line (`@example ...`), whose pipes are its arguments: a `|` ending it carries
    // the command on to the line below, which stays one command (`@search-terms a|\ndef f [] {}`).
    let mut in_attribute = false;
    // The last token was the end of a line right after a redirection without a target, which
    // ends nu's command there but not its pipeline: an item first on the next line is the
    // pipeline's next command (`ls o>\ndef f [] {}`).
    let mut after_open_redirection = false;
    let mut previous = TokenContents::Eol;
    let mut index = 0;
    while let Some(token) = tokens.get(index) {
        index += 1;
        // nu's lite parser takes an `@` item for an attribute line right after an end of line or
        // a `;` only (the start of the block counts as one), in a pipeline too.
        let starts_attribute = matches!(previous, TokenContents::Eol | TokenContents::Semicolon)
            && working_set.get_span_contents(token.span).starts_with('@');
        let blank_line = token.contents == TokenContents::Eol && previous == TokenContents::Eol;
        let continues_pipeline = std::mem::replace(
            &mut after_open_redirection,
            token.contents == TokenContents::Eol
                && matches!(previous, TokenContents::Redirection(operator) if !operator.is_pipe()),
        );
        previous = token.contents;
        match token.contents {
            TokenContents::Eol | TokenContents::Semicolon => {
                at_line_start = true;
                after_statement |= blank_line || token.contents == TokenContents::Semicolon;
            }
            TokenContents::Comment => {}
            TokenContents::Pipe if at_line_start && after_statement => {}
            TokenContents::Item if at_line_start && (after_pipe || continues_pipeline) => {
                // The pipeline's next command, or an attribute line before it.
                at_line_start = false;
                after_statement = false;
                in_attribute = starts_attribute;
                after_pipe = in_attribute;
            }
            TokenContents::Item if at_line_start => {
                at_line_start = false;
                after_statement = false;
                in_attribute = starts_attribute;
                // The items after the head (`index` is past it), with their text. Past an
                // `export`, `words` is left after `def`/`extern`: flags, name, signature.
                let statement = || {
                    tokens[index..]
                        .iter()
                        .take_while(|token| token.contents == TokenContents::Item)
                        .map(|token| (working_set.get_span_contents(token.span), token.span))
                };
                let mut words = statement();
                let (head, items_start) = match working_set.get_span_contents(token.span) {
                    "export" => (words.next().map_or("", |(word, _)| word), index + 1),
                    head => (head, index),
                };
                if !matches!(head, "def" | "extern" | "type") {
                    continue;
                }
                let items_end = items_start
                    + tokens.get(items_start..).map_or(0, |rest| {
                        rest.iter()
                            .take_while(|token| token.contents == TokenContents::Item)
                            .count()
                    });
                let items = tokens.get(items_start..items_end).unwrap_or(&[]);
                // The rest of the statement, to its end of line: nu's lite parser absorbs what
                // follows an `=` into the command (`def 1234 = echo 'x'`).
                let rest_end = items_start
                    + tokens.get(items_start..).map_or(0, |rest| {
                        rest.iter()
                            .take_while(|token| {
                                !matches!(
                                    token.contents,
                                    TokenContents::Eol | TokenContents::Semicolon
                                )
                            })
                            .count()
                    });
                let rest = tokens.get(items_start..rest_end).unwrap_or(&[]);
                // nu predeclares only a pipeline of one command. The definition's command ends at
                // the first pipe of its line, unless an assignment comes first and takes the rest
                // of the line into the command, with the lines a `|` continues it to; without
                // either, a `|` on a later line may go on with the pipeline (`def f [] {}\n| ls`).
                let command_end = rest.iter().position(|token| {
                    is_pipe(token) || matches!(token.contents, TokenContents::AssignmentOperator(_))
                });
                let (command, pipes_to_another_command) = match command_end {
                    Some(at) if is_pipe(&rest[at]) => {
                        (&rest[..at], joins_next_command(items_start + at))
                    }
                    Some(_) => (rest, false),
                    None => {
                        let mut after =
                            Tokens::new(working_set, tokens.get(rest_end..).unwrap_or(&[]), 0);
                        let pipe_below = pipe_on_later_line(&mut after).is_ok();
                        (
                            rest,
                            pipe_below && joins_next_command(rest_end + after.position()),
                        )
                    }
                };
                if pipes_to_another_command {
                    continue;
                }
                if head == "type" {
                    // A name and `=` after the keyword (the `=` is a token of its own).
                    if let [name, equals, ..] = tokens.get(items_start..).unwrap_or(&[])
                        && name.contents == TokenContents::Item
                        && working_set.get_span_contents(equals.span) == "="
                    {
                        let last_part = command
                            .iter()
                            .rev()
                            .find(|token| token.contents != TokenContents::Comment);
                        let span = last_part.map_or(token.span, |last| token.span.merge(last.span));
                        on_type_alias(TypeAliasPredecl {
                            name: name.span,
                            span,
                        });
                    }
                    continue;
                }
                // nu looks at a definition only when its statement has at least three parts.
                let parts = command
                    .iter()
                    .filter(|token| token.contents != TokenContents::Comment)
                    .count();
                let statement_items = parts + if items_start > index { 2 } else { 1 };
                let Some((_, name_span)) = words.find(|(word, _)| !word.starts_with('-')) else {
                    continue;
                };
                if statement_items < 3 {
                    continue;
                }
                let has_signature = words.any(|(word, _)| word.starts_with(['[', '(']));
                let last_part = command
                    .iter()
                    .rev()
                    .find(|token| token.contents != TokenContents::Comment);
                let found = FoundDefinition {
                    span: last_part.map_or(token.span, |last| token.span.merge(last.span)),
                    items,
                    name: Token {
                        contents: TokenContents::Item,
                        span: name_span,
                    },
                    has_signature,
                    wrapped: false,
                    external: head == "extern",
                    declared: None,
                };
                // nu predeclares a definition only when its name item is a string (nu's
                // `parse_string`, which unquotes it) it does not refuse (`reject_command_name`:
                // a parser keyword, `"a  b"`) and a signature item follows it.
                let name = match parse_definition_name(working_set, name_span) {
                    Ok(name)
                        if !name.item.is_empty()
                            && has_signature
                            && check_definition_name(&name, "command").is_ok() =>
                    {
                        name.item
                    }
                    _ => {
                        on_definition(found);
                        continue;
                    }
                };
                // nu gives the untyped rest parameter of a `def --wrapped` the
                // `external_arg` shape.
                let has_wrapped_flag = head == "def"
                    && tokens[index..]
                        .iter()
                        .take_while(|token| token.contents == TokenContents::Item)
                        .any(|token| {
                            token.span.len() == 9
                                && working_set.get_span_contents(token.span) == "--wrapped"
                        });
                let wrapped = has_wrapped_flag
                    && statement()
                        .skip_while(|(_, span)| *span != name_span)
                        .find(|(word, _)| word.starts_with(['[', '(']))
                        .is_some_and(|(_, signature)| has_untyped_rest(working_set, signature));
                let kind = if wrapped {
                    DeclKind::Wrapped
                } else {
                    DeclKind::Declared
                };
                working_set.add_predecl(&name, kind);
                // nu reports the name and goes on, the later definition predeclared (its
                // `add_predecl`), and so does the consumer: the error is recorded as recovered.
                if declared.contains(&name) {
                    working_set.error(
                        Diagnostic::message(
                            "duplicate command definition within a block",
                            name_span,
                        )
                        .with_help(format!("`{name}` is already defined in this block"))
                        .as_recovered(),
                    );
                }
                declared.push(name);
                on_definition(FoundDefinition {
                    wrapped: has_wrapped_flag,
                    declared: Some(kind),
                    ..found
                });
            }
            _ => {
                at_line_start = false;
                after_statement = false;
                if !in_attribute {
                    after_pipe = is_pipe(token) && joins_next_command(index - 1);
                }
            }
        }
    }
}

/// Whether the rest parameter of the signature item at `signature` has no
/// type, as nu decides it. nu takes the parameter from the parsed signature:
/// the first item of the brackets in a parameter's place (not a type after a
/// `:` or a default value after an `=`; comments and quoted values are tokens
/// of their own) that starts with `...`, its name keeping a `$` (`...$r:
/// string` is typed). It is typed when some `...name` in the item's text,
/// comments included, is followed by a `:` (`rest_param_is_type_annotated`).
/// `false` without one.
fn has_untyped_rest(working_set: &WorkingSet<'_>, signature: Span) -> bool {
    let Some(close) = working_set.group_end(signature) else {
        return false;
    };
    let inner = Span::new(signature.start + 1, signature.start + close);
    let Ok(parameters) = working_set.lex(inner, LexOptions::SIGNATURE) else {
        return false;
    };
    let mut after_colon_or_equals = false;
    let name = parameters
        .iter()
        .filter(|token| {
            matches!(
                token.contents,
                TokenContents::Item | TokenContents::AssignmentOperator(_)
            )
        })
        .find_map(|token| {
            let text = working_set.get_span_contents(token.span);
            let in_parameter_place = !after_colon_or_equals;
            after_colon_or_equals = matches!(text, ":" | "=");
            text.strip_prefix("...")
                .filter(|name| in_parameter_place && !name.is_empty())
        });
    let Some(name) = name else { return false };
    let text = working_set.get_span_contents(signature);
    let needle = format!("...{name}");
    !text.match_indices(&needle).any(|(start, _)| {
        text[start + needle.len()..]
            .trim_start_matches(|c: char| c.is_ascii_whitespace())
            .starts_with(':')
    })
}

/// Reject a `def`/`extern`/`alias` name that is a parser keyword, or that
/// nu refuses because it could never be called: one containing `#`, `^` or
/// `%`, or one that Rust reads as a float (`1e3`, `inf`) or the `bytesize`
/// crate as a size (`1k`, `2.5gib`, `"1 kb"`), or one whose whitespace is not
/// single spaces between words (a call is looked up by its words joined with
/// single spaces, so `"foo  bar"` or `" foo"` could never be called). Nushell's
/// own number syntax does not count: `def 0x10` and `def 1_000` are fine.
pub fn check_definition_name(name: &Spanned<Cow<'_, str>>, what: &str) -> ParseResult<()> {
    if is_parser_keyword(&name.item) {
        return Err(cut(Diagnostic::message(
            format!("cannot use parser keyword `{}` as {what} name", name.item),
            name.span,
        )
        .with_help(
            "choose a different name; this word is parsed specially by Nushell",
        )));
    }
    let text: &str = &name.item;
    let single_spaced = !text.starts_with(|c: char| c.is_ascii_whitespace())
        && !text.ends_with(|c: char| c.is_ascii_whitespace())
        && !text.contains("  ")
        && text
            .bytes()
            .all(|byte| byte == b' ' || !byte.is_ascii_whitespace());
    if text.contains(['#', '^', '%'])
        || is_byte_size(text)
        || text.parse::<f64>().is_ok()
        || !single_spaced
    {
        return Err(cut(Diagnostic::message(format!("{what} name not supported"), name.span).with_help(
            "a name may not contain `#`, `^` or `%`, read as a number or filesize, or use whitespace other than single spaces between words",
        )));
    }
    Ok(())
}

/// A `def`'s or `extern`'s name that [`check_definition_name`] refuses: nu's predeclaration
/// and `finish_def`/`finish_extern` report it (CommandDefNotValid, NameIsKeyword) and go on
/// with the definition's call, and so does the consumer: the error is recorded as recovered.
fn recover_name_error(working_set: &WorkingSet<'_>, result: ParseResult<()>) -> ParseResult<()> {
    match result {
        Err(ErrMode::Cut(ParseFailure::Error(diagnostic))) => {
            working_set.error(diagnostic.as_recovered());
            Ok(())
        }
        other => other,
    }
}

/// Whether the `bytesize` crate (`ByteSize::from_str`, which nu asks about a
/// definition name) parses `text`: an integer, or digits and dots making a
/// float, then optional whitespace and a size unit in any case (`b`, `k`,
/// `kb`, `ki`, `kib`, and the same for `m`, `g`, `t`, `p` and `e`).
fn is_byte_size(text: &str) -> bool {
    const UNITS: [&str; 25] = [
        "b", "k", "kb", "m", "mb", "g", "gb", "t", "tb", "p", "pb", "e", "eb", "ki", "kib", "mi",
        "mib", "gi", "gib", "ti", "tib", "pi", "pib", "ei", "eib",
    ];
    if text.parse::<u64>().is_ok() {
        return true;
    }
    let number_end = text
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(text.len());
    let (number, unit) = text.split_at(number_end);
    number.parse::<f64>().is_ok()
        && UNITS
            .iter()
            .any(|known| unit.trim_start().eq_ignore_ascii_case(known))
}

/// A bare `def`/`extern` name that nu does not take for a name: one starting
/// with `-` (a lone `-`; the others are flags), for which nu takes the next
/// item that does not start with `-` (`def - [] {}` has "no space between name
/// and parameters", `extern - []` predeclares a command it then cannot find),
/// and a spread (`def ...{x: 1} [] {}`), which is no positional.
fn reject_name_item(working_set: &WorkingSet<'_>, token: &Token) -> ParseResult<()> {
    let text = working_set.get_span_contents(token.span);
    if text.starts_with('-') || is_spread(text, b"[{$(") {
        return Err(cut(Diagnostic::message(
            "command name not supported",
            token.span,
        )
        .with_help(
            "a bare command name cannot start with `-` or be a spread; quote it",
        )));
    }
    Ok(())
}

/// Whether the item nu's predeclaration of a `def` or `extern` with a lone `-` for its name
/// takes for that name, the next item not starting with `-` (`decl_name_span`), has a `[` or
/// `(`: nu then reports "no space between name and parameters" (`detect_params_in_name`, a
/// semantic error) and reads no call (`def - [] {}`). Only such an item can start a signature
/// that reads, so after any other the signature nu's call reads is an error.
fn params_in_dash_name(tokens: &Tokens<'_, '_>) -> bool {
    tokens
        .remaining()
        .iter()
        .filter(|token| token.contents == TokenContents::Item)
        .map(|token| tokens.text(token))
        .find(|text| !text.starts_with('-'))
        .is_some_and(|text| text.contains(['[', '(']))
}

/// The name item of a `def`/`extern`: a string literal. Like nu, a name
/// containing `[` or `(` (even quoted) is "no space between name and
/// parameters", and a `$` item is not a string.
fn parse_def_name<'a>(
    working_set: &WorkingSet<'a>,
    token: &Token,
) -> ParseResult<Spanned<Cow<'a, str>>> {
    let text = working_set.get_span_contents(token.span);
    if let Some(at) = text.find(['[', '(']) {
        let span = Span::point(token.span.start + at);
        return Err(cut(Diagnostic::message(
            "no space between name and parameters",
            span,
        )
        .with_help(
            "consider adding a space between the command's name and its parameters",
        )));
    }
    if text.starts_with('$') {
        return Err(cut(Diagnostic::expected("string", token.span)
            .with_help("the name of a definition is a string")));
    }
    parse_definition_name(working_set, token.span)
}

/// `def [--env] [--wrapped] name signature [: input/output types] { body }`;
/// nu also accepts the flags after the name.
pub fn parse_def<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    if reads_as_call(&tokens, DEF_FLAGS, true) {
        return parse_help_call(tokens);
    }
    let statement = tokens;
    let mut call = KeywordCall::start(&mut tokens)?;
    let mut flags = parse_definition_flags(&mut tokens, &mut call, "def", DEF_FLAGS)?;
    let Some(name) = call.positional(&mut tokens, "command name")? else {
        return call.help_call();
    };
    // A lone `-` is the name for nu's call, which then reads the signature from the next item
    // (`def - foo [] {}` has "expected colon (:) before type signature"), its error recorded
    // below as any signature's; the statement is that call ([`params_in_dash_name`]).
    let lone_dash = tokens.text(&name) == "-";
    if lone_dash && params_in_dash_name(&tokens) {
        return call.help_call();
    }
    let name = if lone_dash {
        Spanned::new(Cow::Borrowed("-"), name.span)
    } else {
        // A spread nu's call reads as no name, an error it goes on past.
        if let Err(error) = reject_name_item(working_set, &name) {
            return signature_error_call(statement, error);
        }
        let name = match parse_def_name(working_set, &name) {
            Ok(parsed) => parsed,
            // `def cmd[] {}`: nu refuses the name before it reads the call, and so does the
            // consumer, which reads the statement as that call. A `$` item is no string for
            // the call's `string` positional (`def $x`: VariableNotFound, or `finish_def`'s
            // UnknownState for a value that is no literal), which nu reports and goes on past.
            Err(error)
                if working_set
                    .get_span_contents(name.span)
                    .contains(['[', '('])
                    || working_set.get_span_contents(name.span).starts_with('$') =>
            {
                return signature_error_call(statement, error);
            }
            Err(error) => return Err(error),
        };
        recover_name_error(working_set, check_definition_name(&name, "command"))?;
        name
    };
    flags.extend(parse_definition_flags(
        &mut tokens,
        &mut call,
        "def",
        DEF_FLAGS,
    )?);
    call.flags(&mut tokens)?;
    // The signature positional takes every remaining item but the last, which
    // is the body's: `def foo [] {} --help` drops the `{}` (two-item form of
    // `parse_full_signature`) and `def foo [] {} {} --help` has no colon.
    let (signature_items, body) = match tokens.remaining() {
        [] => {
            call.missing();
            return call.help_call();
        }
        [only] => (std::slice::from_ref(only), None),
        [signature_items @ .., body] => (signature_items, Some(body)),
    };
    let signature = match parse_full_signature(working_set, signature_items, false) {
        Ok(signature) => signature,
        Err(error) => return signature_error_call(statement, error),
    };
    let (body_params, body_block) = match body {
        Some(token) if is_help_flag(working_set, token) && !seen_end_of_options(&tokens) => {
            // nu never looks at the brace after a lone signature item (`def foo
            // [] {}{} --help`), so its help call does not parse it either.
            return match signature_items {
                [_, dropped] if working_set.get_span_contents(dropped.span).starts_with('{') => {
                    call.help_call_without(dropped)
                }
                _ => call.help_call(),
            };
        }
        // nu's call reports a body that is no closure (`def a [] (echo 4)`) and goes on.
        Some(token) => {
            match parse_def_body(working_set, token, "definition body closure { ... }") {
                Ok(body) => body,
                Err(error) => return signature_error_call(statement, error),
            }
        }
        None => {
            call.missing();
            return call.help_call();
        }
    };
    if lone_dash {
        return call.help_call();
    }
    let span = call
        .keyword
        .span
        .merge(body.map_or(signature.span, |token| token.span));
    let def = Def {
        flags,
        name,
        signature,
        body_params,
        body: body_block,
    };
    call.finish(Expression::new(Expr::Def(Box::new(def)), span))
}

/// A `def` or `extern` `statement` whose signature [`parse_full_signature`] refused with
/// `error`: nu's `parse_def` and `parse_extern` read the signature as their call's argument
/// with nu's `parse_full_signature`, which reports its errors and goes on, and finish the
/// definition with what it gives. The statement is then that call ([`call_after_error`]).
fn signature_error_call<'a>(
    statement: Tokens<'_, 'a>,
    error: ErrMode<ParseFailure>,
) -> ParseResult<Expression<'a>> {
    call_after_error(statement, error, parse_definition_call)
}

/// The body of a `def`: nu parses it as a closure without looking at its
/// shape first (`def f [] {a: 1}` calls `a:`), and drops its parameters.
fn parse_def_body<'a>(
    working_set: &WorkingSet<'a>,
    token: &Token,
    what: &'static str,
) -> ParseResult<(Option<Signature<'a>>, Block<'a>)> {
    if token.contents != TokenContents::Item
        || !working_set.get_span_contents(token.span).starts_with('{')
    {
        return Err(cut(Diagnostic::expected(what, token.span)));
    }
    match brace_shape(working_set, token.span)? {
        BraceShape::ClosureParams => {
            let closure = parse_closure_parts(working_set, token.span)?;
            Ok((closure.params, closure.body))
        }
        _ => Ok((None, parse_block_body_unchecked(working_set, token.span)?)),
    }
}

/// Whether nu's call to `def` or `extern` (the statement `tokens`, whose keyword's flags `known`
/// lists) reports an error of its own, a call's: then the statement is that call
/// ([`parse_help_call`]) for the consumer to read, as nu's `finish_def` and `finish_extern` read
/// it. That is a flag before the signature that is not one of `known` (`def --foo f [] {}`,
/// `def -e f [] {}`, `def -1 f [] {}`), or, for a `def` (`body`), a flag or a `--` in the body's
/// place, its last item, whose signature then takes the body (`def f [] {} --env` has no body)
/// when those items still read as one (`def f [] {} {} --env` is "colon before type
/// signature", an error here too). A flag between the signature and the body is the
/// signature's (nu's `calculate_end_span` gives it every item but the last), an error there.
fn reads_as_call<Flag>(tokens: &Tokens<'_, '_>, known: &[(&str, Flag)], body: bool) -> bool {
    let working_set = tokens.working_set;
    let items: Vec<Token> = tokens
        .remaining()
        .iter()
        .skip(1)
        .filter(|token| token.contents == TokenContents::Item)
        .copied()
        .collect();
    // The items of a signature that takes everything from its start to the last item.
    let signature_reads = |start: Option<usize>| match start {
        Some(start) if start < items.len() - 1 => {
            parse_full_signature(working_set, &items[start..items.len() - 1], false).is_ok()
        }
        _ => true,
    };
    let mut positionals = 0;
    let mut signature_start = None;
    for (index, token) in items.iter().enumerate() {
        let text = working_set.get_span_contents(token.span);
        // nu looks for no flag after a `--`; one in the body's place leaves the body out.
        if text == "--" {
            return body && index == items.len() - 1 && signature_reads(signature_start);
        }
        if is_help_text(text) {
            continue;
        }
        if text.len() < 2 || !text.starts_with('-') {
            positionals += 1;
            if positionals == 2 {
                signature_start = Some(index);
            }
            continue;
        }
        let name = text.split('=').next().unwrap_or(text);
        if positionals < 2 && !known.iter().any(|(known, _)| *known == name) {
            return true;
        }
        if body && index == items.len() - 1 {
            return signature_reads(signature_start);
        }
    }
    false
}

/// The flags of a `def`.
const DEF_FLAGS: &[(&str, DefFlag)] = &[("--env", DefFlag::Env), ("--wrapped", DefFlag::Wrapped)];

/// The flags of an `extern`.
const EXTERN_FLAGS: &[(&str, ExternFlag)] = &[("--strict", ExternFlag::Strict)];

/// The flags of a `def` or an `extern` (its keyword), each named as `known`
/// lists it, up to a `--` or an item that is not a long flag. Help flags
/// among them go to `call` (`def --help --env f`).
fn parse_definition_flags<'t, 'a, Flag: Copy>(
    tokens: &mut Tokens<'t, 'a>,
    call: &mut KeywordCall<'t, 'a>,
    keyword: &str,
    known: &[(&str, Flag)],
) -> ParseResult<Vec<DefinitionFlag<'a, Flag>>> {
    let mut flags = Vec::new();
    loop {
        if call.help_flag(tokens) {
            continue;
        }
        match opt(|tokens: &mut Tokens<'_, 'a>| definition_flag(tokens, keyword, known))
            .parse_next(tokens)?
        {
            Some(flag) => flags.push(flag),
            None => return Ok(flags),
        }
    }
}

/// One of the `known` flags, with its value when it is written `--env=false`. Backtracks at a help flag, a `--` or an item that
/// is not a long flag; any other long flag is an error.
fn definition_flag<'a, Flag: Copy>(
    tokens: &mut Tokens<'_, 'a>,
    keyword: &str,
    known: &[(&str, Flag)],
) -> ParseResult<DefinitionFlag<'a, Flag>> {
    let working_set = tokens.working_set;
    let token = item
        .verify(|token| {
            let text = working_set.get_span_contents(token.span);
            text.starts_with("--") && text != "--" && !is_help_text(text)
        })
        .parse_next(tokens)?;
    let text = tokens.text(&token);
    let (name, value_span) = split_long_flag(text, token.span);
    let Some((_, flag)) = known
        .iter()
        .find(|(known, _)| known.strip_prefix("--") == Some(name))
    else {
        let accepted: Vec<String> = known.iter().map(|(name, _)| format!("`{name}`")).collect();
        return Err(cut(Diagnostic::message(
            format!("the `{keyword}` command doesn't have flag `--{name}`"),
            token.span,
        )
        .with_help(format!("`{keyword}` accepts {}", accepted.join(" and ")))));
    };
    // nu reads the value as the switch's `bool`; one of another type is the consumer's to
    // report (`def --env=1`).
    let value = match value_span {
        Some(span) => Some(Box::new(parse_flag_value(
            working_set,
            span,
            |working_set, span| parse_value(working_set, span, ExpectedShape::Any),
        )?)),
        None => None,
    };
    Ok(DefinitionFlag {
        span: token.span,
        flag: *flag,
        value,
    })
}

/// `extern [--strict] name signature [: input/output types]`; nu also
/// accepts the flag after the name.
pub fn parse_extern<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    if reads_as_call(&tokens, EXTERN_FLAGS, false) {
        return parse_help_call(tokens);
    }
    let statement = tokens;
    let mut call = KeywordCall::start(&mut tokens)?;
    let mut flags = parse_definition_flags(&mut tokens, &mut call, "extern", EXTERN_FLAGS)?;
    let Some(name) = call.positional(&mut tokens, "command name")? else {
        return call.help_call();
    };
    // A lone `-` is the name for nu's call, which then reads the signature from the next item
    // (`def - foo [] {}` has "expected colon (:) before type signature"), its error recorded
    // below as any signature's; the statement is that call ([`params_in_dash_name`]).
    let lone_dash = tokens.text(&name) == "-";
    if lone_dash && params_in_dash_name(&tokens) {
        return call.help_call();
    }
    let name = if lone_dash {
        Spanned::new(Cow::Borrowed("-"), name.span)
    } else {
        // A spread nu's call reads as no name, an error it goes on past.
        if let Err(error) = reject_name_item(working_set, &name) {
            return signature_error_call(statement, error);
        }
        let name = match parse_def_name(working_set, &name) {
            Ok(parsed) => parsed,
            // `def cmd[] {}`: nu refuses the name before it reads the call, and so does the
            // consumer, which reads the statement as that call. A `$` item is no string for
            // the call's `string` positional (`def $x`: VariableNotFound, or `finish_def`'s
            // UnknownState for a value that is no literal), which nu reports and goes on past.
            Err(error)
                if working_set
                    .get_span_contents(name.span)
                    .contains(['[', '('])
                    || working_set.get_span_contents(name.span).starts_with('$') =>
            {
                return signature_error_call(statement, error);
            }
            Err(error) => return Err(error),
        };
        recover_name_error(working_set, check_definition_name(&name, "command"))?;
        name
    };
    flags.extend(parse_definition_flags(
        &mut tokens,
        &mut call,
        "extern",
        EXTERN_FLAGS,
    )?);
    call.flags(&mut tokens)?;
    let rest = tokens.remaining();
    let Some(last) = rest.last() else {
        call.missing();
        return call.help_call();
    };
    // The signature argument takes every remaining item, so a body after it
    // (the old `extern-wrapped`) is dropped by nu without a look.
    let signature = match parse_full_signature(working_set, rest, true) {
        Ok(signature) => signature,
        Err(error) => return signature_error_call(statement, error),
    };
    if lone_dash {
        return call.help_call();
    }
    let span = call.keyword.span.merge(last.span);
    call.finish(Expression::new(
        Expr::Extern(Box::new(Extern {
            flags,
            name,
            signature,
        })),
        span,
    ))
}

/// `for variable[: type] in iterable { block }`.
pub fn parse_for<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let mut call = KeywordCall::start(&mut tokens)?;
    let Some(variable) = call.positional(&mut tokens, "loop variable")? else {
        return call.help_call();
    };
    // nu's call to `for` takes a first `in` as its keyword, the variable missing before it
    // (`for in in [] {}`), not as a reserved name.
    if tokens.text(&variable) == "in" {
        call.missing();
        return call.help_call();
    }
    let (var, typed) = match parse_var_with_opt_type(working_set, &variable) {
        Ok(parsed) => parsed,
        // nu's call reports an untyped name it refuses (`for x.y in [1] {}`) and goes on with
        // garbage for the variable.
        Err(ErrMode::Cut(ParseFailure::Error(diagnostic)))
            if !tokens.text(&variable).ends_with(':') =>
        {
            return call.call_after(*diagnostic);
        }
        Err(error) => return Err(error),
    };
    // The variable positional runs up to the `in` keyword, so a type after
    // `x:` is every item before it (`record<a: int, b: string>` is four items)
    // and `for x: int --help in [] {}` has the unknown type `int --help`.
    let ty = match typed {
        true => {
            // Without an `in`, nu gives the type only the items the later positionals leave
            // (`calculate_end_span`), none in `for x: int` (MissingType), and reads the rest as
            // those positionals, going on past its errors: the statement is that call, the
            // error recorded as recovered.
            if !tokens
                .remaining()
                .iter()
                .any(|token| tokens.text(token) == "in")
            {
                return call.call_after(Diagnostic::expected("type", variable.span.past()));
            }
            let Some(type_span) = tokens_until("in").parse_next(&mut tokens)?.span() else {
                return Err(cut(Diagnostic::expected("type", variable.span.past())));
            };
            Some(parse_var_type(working_set, type_span)?)
        }
        false => None,
    };
    let Some(in_keyword) = call.positional(&mut tokens, "`in`")? else {
        return call.help_call();
    };
    // nu reports another word (ExpectedKeyword) and goes on with its call.
    if tokens.text(&in_keyword) != "in" {
        return call.call_after(Diagnostic::new(
            ErrorKind::ExpectedKeyword("in"),
            in_keyword.span,
        ));
    }
    // nu reserves the last item for the block before it parses the keyword's
    // argument, so `for x in []` and `for x --help in []` lack the argument of
    // `in` (KeywordMissingArgument), help or not, which nu reports and goes on past.
    if tokens.remaining().len() < 2 {
        return call.call_after(
            Diagnostic::message("missing argument to `in`", in_keyword.span)
                .with_help("`for` needs a value to iterate and a block: `for x in [1 2] { }`"),
        );
    }
    let iterable = tokens.expect_item("value to iterate")?;
    let iterable = Box::new(parse_value(working_set, iterable.span, ExpectedShape::Any)?);
    let Some(block) = call.positional(&mut tokens, "block")? else {
        return call.help_call();
    };
    // An item that is no block nu's call reports and goes on past (`for x in -h [1] {}`).
    let (body, body_value) = match parse_block_or_value_argument(working_set, &block, "block") {
        Ok(parsed) => parsed,
        Err(ErrMode::Cut(ParseFailure::Error(diagnostic))) => return call.call_after(*diagnostic),
        Err(error) => return Err(error),
    };
    call.end(&mut tokens)?;
    let span = call.keyword.span.merge(block.span);
    let for_loop = For {
        var,
        ty,
        in_keyword: in_keyword.span,
        iterable,
        body,
        body_value,
    };
    call.finish(Expression::new(Expr::For(Box::new(for_loop)), span))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::winnow::syntax::parser::ParseConfig;

    /// How the predeclaration scan of `source` declared `name`, if it did.
    fn declared(source: &str, name: &str) -> Option<DeclKind> {
        let working_set = WorkingSet::new(source, &ParseConfig::empty());
        let tokens = working_set
            .lex(Span::new(0, source.len()), LexOptions::BLOCK)
            .unwrap();
        parse_def_predecl(
            &working_set,
            Tokens::from_lexed(&working_set, &tokens).all(),
        );
        working_set.find_decl(name)
    }

    #[test]
    fn definition_with_a_dropped_pipe_is_declared() {
        // nu drops a `|` that a blank line, the end of the block or, after an `e>|`, the end of
        // its line follows: the definition is a pipeline of its own.
        for source in [
            "def f [] {} |\n\nf",
            "def f [] {} | # c\n\nf",
            "def f [] {} |\n",
            "def f [] {}\n|\n\nf",
            "def f [] {} | |\n\nf",
            "def f [] {} e>|\nf",
            "^ls e>|\ndef f [] {}",
            "^ls o+e>| # c\ndef f [] {}",
        ] {
            assert_eq!(
                declared(source, "f"),
                Some(DeclKind::Declared),
                "{source:?}"
            );
        }
        assert_eq!(
            declared("extern e [] |\n\ne", "e"),
            Some(DeclKind::Declared)
        );
    }

    #[test]
    fn definition_in_a_longer_pipeline_is_not_declared() {
        for source in [
            "def f [] {} | ls",
            "def f [] {} |\n# c\nls",
            "def f [] {} |\n\n| ls",
            "def f [] {}\n| ls",
            "ls |\n\n| def f [] {}",
            "ls |\n# c\n\n# d\n| def f [] {}",
            "^ls e>|\n# c\n| def f [] {}",
            "ls | length |\n@search-terms foo\ndef f [] {}",
        ] {
            assert_eq!(declared(source, "f"), None, "{source:?}");
        }
    }

    #[test]
    fn attribute_lines_keep_their_definition() {
        for source in [
            "|\n@search-terms foo\ndef f [] {}",
            "@search-terms foo\n| bar\ndef f [] {}",
        ] {
            assert_eq!(
                declared(source, "f"),
                Some(DeclKind::Declared),
                "{source:?}"
            );
        }
    }

    #[test]
    fn rest_parameter_of_a_wrapped_definition() {
        // The rest parameter is the signature's, not a `...word` in a comment or a default value
        // before it; a `...name:` anywhere in the text makes it typed, as in nu.
        for (source, kind) in [
            ("def --wrapped g [...rest] {}", DeclKind::Wrapped),
            (
                "def --wrapped g [\n  # usage: g ...flags\n  ...args: string\n] {}",
                DeclKind::Declared,
            ),
            (
                "def --wrapped g [--x: string = \"a ...b\", ...rest: string] {}",
                DeclKind::Declared,
            ),
            (
                "def --wrapped g [\n  ...rest # see ...rest: list\n] {}",
                DeclKind::Declared,
            ),
            ("def --wrapped g [...$r: string] {}", DeclKind::Declared),
            ("def --wrapped g [x # ...y\n ...rest] {}", DeclKind::Wrapped),
        ] {
            assert_eq!(declared(source, "g"), Some(kind), "{source:?}");
        }
    }

    #[test]
    fn definition_name_is_its_string() {
        let source = "def --wrapped \"'echo'\" [...rest] {}";
        assert_eq!(declared(source, "'echo'"), Some(DeclKind::Wrapped));
        assert_eq!(declared(source, "echo"), None);
        assert_eq!(declared("def \"a\"b [] {}", "a\"b"), None);
    }
}
