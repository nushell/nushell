//! `def`, `extern`, `for`, attribute blocks and the predeclaration of
//! commands (nu-parser's `parse_def.rs`).

use std::borrow::Cow;

use winnow::Parser;
use winnow::combinator::repeat;

use crate::ast::{Block, Def, DefFlag, Expr, Expression, Extern, For, Signature};
use crate::error::{Diagnostic, ErrorKind};
use crate::input::{ParseResult, cut};
use crate::lex::{Token, TokenContents};
use crate::span::{Span, Spanned};

use super::WorkingSet;
use super::lite_parser::pipe_on_later_line;
use super::parse_expressions::{
    BraceShape, ExpectedShape, brace_shape, parse_block_body_unchecked, parse_closure_parts, parse_value,
};
use super::parse_helpers::{is_help_flag, is_spread};
use super::parse_keywords::{KeywordCall, is_parser_keyword, parse_block_or_value_argument, seen_end_of_options};
use super::parse_signatures::{parse_definition_name, parse_full_signature, parse_var_type, parse_var_with_opt_type};
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
    scan_definitions(working_set, tokens, |_| {});
}

/// A `def` or `extern` of a block found by the predeclaration scan, with its
/// signature parsed: what an engine needs to declare the command before the
/// block's statements are parsed (see [`crate::BlockSink::predecl`]). Every
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
/// names, so they can be parsed meanwhile (see [`crate::BlockStatements::new`]).
pub(super) fn scan_predecls(working_set: &WorkingSet<'_>, tokens: &[Token]) -> Definitions {
    let mut found = Vec::new();
    scan_definitions(working_set, tokens, |definition| {
        // Copied, so that `Definitions` borrows nothing.
        let items = definition.items.to_vec();
        found.push(definition.with_items(items));
    });
    Definitions(found)
}

/// The `def`/`extern` statements of a block, as the predeclaration scan found
/// them (see [`crate::BlockStatements::new`]), their signatures not yet
/// parsed.
#[derive(Debug, Default)]
pub struct Definitions(Vec<FoundDefinition<Vec<Token>>>);

impl Definitions {
    /// Each definition with its signature, parsed in `source` with the
    /// commands `lookup` knows. A signature is parsed as the scan would have
    /// parsed it: with the names of the block's definitions up to and
    /// including its own declared. What parsing a name or signature reports is
    /// dropped, as nu's predeclaration drops it (the statement reports it
    /// again when parsed).
    pub fn parse<'a>(self, source: &'a str, span: Span, lookup: impl CommandLookup + 'a) -> Vec<PredeclaredDef<'a>> {
        let working_set = WorkingSet::with_lookup(source, span, lookup);
        self.0
            .into_iter()
            .map(|found| {
                // A new working set: the scan's names are declared again, in its order.
                if let Some(kind) = found.declared {
                    working_set.add_predecl(declared_name(&working_set, found.name.span), kind);
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
        let FoundDefinition { span, name, has_signature, wrapped, external, declared, .. } = *self;
        FoundDefinition { span, items, name, has_signature, wrapped, external, declared }
    }
}

/// The name a definition whose name item is at `span` is declared under: its text, unquoted.
fn declared_name<'a>(working_set: &WorkingSet<'a>, span: Span) -> &'a str {
    working_set.get_span_contents(span).trim_matches(['"', '\'', '`'])
}

/// A definition found by the scan, with its signature: the items from the
/// first one starting with `[` or `(` after the name, up to (not including) a
/// `def`'s body.
fn predeclared_def<'a>(working_set: &WorkingSet<'a>, found: FoundDefinition<&[Token]>) -> PredeclaredDef<'a> {
    let name = match parse_definition_name(working_set, found.name.span) {
        Ok(name) => name,
        Err(_) => Spanned::new(Cow::Borrowed(working_set.get_span_contents(found.name.span)), found.name.span),
    };
    let signature = found.has_signature.then(|| {
        let after_name = found.items.iter().position(|token| token.span == found.name.span)? + 1;
        let signature_start = after_name
            + found.items[after_name..]
                .iter()
                .position(|token| working_set.get_span_contents(token.span).starts_with(['[', '(']))?;
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
    PredeclaredDef { span: found.span, name, signature, wrapped: found.wrapped, external: found.external }
}

/// The predeclaration scan: declare every `def`/`extern` of a block that has a
/// signature item after its name, report a name declared twice, and hand each
/// definition with a name (a statement of at least three items, as nu
/// requires) to `on_definition`.
///
/// It runs before the lite parse, over the block's tokens, so it finds where
/// statements start itself: at an item first on its line or after a `;`, past
/// a `|` that leads a new statement but not one that continues a pipeline. Of
/// each statement it reads only the head and the tokens up to its end of line.
fn scan_definitions(
    working_set: &WorkingSet<'_>,
    tokens: &[Token],
    mut on_definition: impl FnMut(FoundDefinition<&[Token]>),
) {
    let mut declared: Vec<&str> = Vec::new();
    // A `|`, or a redirection into one (`e>|`), joins the next command to the pipeline.
    let joins_next_command = |token: &Token| match token.contents {
        TokenContents::Pipe => true,
        TokenContents::Redirection(operator) => operator.is_pipe(),
        _ => false,
    };
    // The next item is the first of its line, or the first after a `;`.
    let mut at_line_start = true;
    // No command since the start of the block, a `;` or a blank line: a `|` here leads the next
    // command (`|def x [] {}`) instead of joining a pipeline.
    let mut after_statement = true;
    // The last token but comments and one end of line was a `|` that joins a pipeline, so the
    // next item continues that pipeline (`ls |\ndef f [] {}`), even first on its line.
    let mut after_pipe = false;
    // The statement started with an attribute (`@example ...`): a `|` ending its line carries
    // it on to the command below, which stays one command (`@search-terms a|\ndef f [] {}`).
    let mut in_attribute = false;
    let mut previous = TokenContents::Eol;
    let mut index = 0;
    while let Some(token) = tokens.get(index) {
        index += 1;
        let blank_line = token.contents == TokenContents::Eol && previous == TokenContents::Eol;
        previous = token.contents;
        match token.contents {
            TokenContents::Eol | TokenContents::Semicolon => {
                at_line_start = true;
                after_statement |= blank_line || token.contents == TokenContents::Semicolon;
                // A blank line or a `;` ends the pipeline (nu's `after_pipe` finds it dangling).
                after_pipe &= !after_statement;
            }
            TokenContents::Comment => {}
            TokenContents::Pipe if at_line_start && after_statement => {}
            TokenContents::Item if at_line_start && !after_pipe => {
                at_line_start = false;
                after_statement = false;
                in_attribute = working_set.get_span_contents(token.span).starts_with('@');
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
                if !matches!(head, "def" | "extern") {
                    continue;
                }
                let items_end = items_start
                    + tokens.get(items_start..).map_or(0, |rest| {
                        rest.iter().take_while(|token| token.contents == TokenContents::Item).count()
                    });
                let items = tokens.get(items_start..items_end).unwrap_or(&[]);
                // The rest of the statement, to its end of line: nu's lite parser absorbs what
                // follows an `=` into the command (`def 1234 = echo 'x'`).
                let rest_end = items_start
                    + tokens.get(items_start..).map_or(0, |rest| {
                        rest.iter()
                            .take_while(|token| {
                                !matches!(token.contents, TokenContents::Eol | TokenContents::Semicolon)
                            })
                            .count()
                    });
                let rest = tokens.get(items_start..rest_end).unwrap_or(&[]);
                // nu predeclares only a pipeline of one command (an assignment takes the pipes
                // after it into its command), and a `|` on a later line joins the next command
                // (`def f [] {}\n| ls`).
                let continues_on_a_later_line = tokens
                    .get(rest_end..)
                    .is_some_and(|after| pipe_on_later_line(&mut Tokens::new(working_set, after, 0)).is_ok());
                let pipes_to_another_command = continues_on_a_later_line
                    || rest
                        .iter()
                        .take_while(|token| !matches!(token.contents, TokenContents::AssignmentOperator(_)))
                        .any(joins_next_command);
                if pipes_to_another_command {
                    continue;
                }
                // nu looks at a definition only when its statement has at least three parts.
                let parts = rest.iter().filter(|token| token.contents != TokenContents::Comment).count();
                let statement_items = parts + if items_start > index { 2 } else { 1 };
                let Some((name, name_span)) = words.find(|(word, _)| !word.starts_with('-')) else { continue };
                if statement_items < 3 {
                    continue;
                }
                let has_signature = words.any(|(word, _)| word.starts_with(['[', '(']));
                let last_part = rest.iter().rev().find(|token| token.contents != TokenContents::Comment);
                let found = FoundDefinition {
                    span: last_part.map_or(token.span, |last| token.span.merge(last.span)),
                    items,
                    name: Token { contents: TokenContents::Item, span: name_span },
                    has_signature,
                    wrapped: false,
                    external: head == "extern",
                    declared: None,
                };
                let name = name.trim_matches(['"', '\'', '`']);
                // nu predeclares a definition only when a signature item follows the name.
                if name.is_empty() || !has_signature {
                    on_definition(found);
                    continue;
                }
                // nu gives the untyped rest parameter of a `def --wrapped` the
                // `external_arg` shape.
                let has_wrapped_flag = head == "def"
                    && tokens[index..]
                        .iter()
                        .take_while(|token| token.contents == TokenContents::Item)
                        .any(|token| token.span.len() == 9 && working_set.get_span_contents(token.span) == "--wrapped");
                let wrapped = has_wrapped_flag
                    && statement()
                        .skip_while(|(_, span)| *span != name_span)
                        .find(|(word, _)| word.starts_with(['[', '(']))
                        .is_some_and(|(signature, _)| has_untyped_rest(signature));
                let kind = if wrapped { DeclKind::Wrapped } else { DeclKind::Declared };
                working_set.add_predecl(name, kind);
                if declared.contains(&name) {
                    working_set.error(
                        Diagnostic::message("duplicate command definition within a block", name_span)
                            .with_help(format!("`{name}` is already defined in this block")),
                    );
                }
                declared.push(name);
                on_definition(FoundDefinition { wrapped: has_wrapped_flag, declared: Some(kind), ..found });
            }
            _ => {
                at_line_start = false;
                after_statement = false;
                after_pipe = !in_attribute && joins_next_command(token);
            }
        }
    }
}

/// Whether the rest parameter of a signature's text has no type, as nu
/// decides it (`rest_param_is_type_annotated`, on the text): the parameter's
/// name is the word after the `...` that starts it (`$` included, so `...$r:
/// string` is typed), and it is typed when some `...name` is followed by a
/// `:`. `false` without one.
fn has_untyped_rest(signature: &str) -> bool {
    let name = signature.match_indices("...").find_map(|(start, _)| {
        let starts_parameter = signature[..start]
            .chars()
            .next_back()
            .is_none_or(|previous| previous.is_whitespace() || matches!(previous, '[' | '(' | ','));
        let rest = &signature[start + 3..];
        let end = rest
            .find(|next: char| next.is_whitespace() || matches!(next, ',' | ':' | ']' | ')' | '#' | '=' | '?'))
            .unwrap_or(rest.len());
        (starts_parameter && end > 0).then(|| &rest[..end])
    });
    let Some(name) = name else { return false };
    let needle = format!("...{name}");
    !signature.match_indices(&needle).any(|(start, _)| {
        signature[start + needle.len()..].trim_start_matches(|c: char| c.is_ascii_whitespace()).starts_with(':')
    })
}

/// Reject a `def`/`extern`/`alias` name that is a parser keyword, or that
/// nu refuses because it could never be called: one containing `#`, `^` or
/// `%`, or one that Rust reads as a float (`1e3`, `inf`) or the `bytesize`
/// crate as a size (`1k`, `2.5gib`, `"1 kb"`). Nushell's own number syntax
/// does not count: `def 0x10` and `def 1_000` are fine.
pub fn check_definition_name(name: &Spanned<Cow<'_, str>>, what: &str) -> ParseResult<()> {
    if is_parser_keyword(&name.item) {
        return Err(cut(Diagnostic::message(
            format!("cannot use parser keyword `{}` as {what} name", name.item),
            name.span,
        )
        .with_help("choose a different name; this word is parsed specially by Nushell")));
    }
    let text: &str = &name.item;
    if text.contains(['#', '^', '%']) || is_byte_size(text) || text.parse::<f64>().is_ok() {
        return Err(cut(Diagnostic::message(format!("{what} name not supported"), name.span)
            .with_help("a name may not contain `#`, `^` or `%`, or read as a number or filesize")));
    }
    Ok(())
}

/// Whether the `bytesize` crate (`ByteSize::from_str`, which nu asks about a
/// definition name) parses `text`: an integer, or digits and dots making a
/// float, then optional whitespace and a size unit in any case (`b`, `k`,
/// `kb`, `ki`, `kib`, and the same for `m`, `g`, `t`, `p` and `e`).
fn is_byte_size(text: &str) -> bool {
    const UNITS: [&str; 25] = [
        "b", "k", "kb", "m", "mb", "g", "gb", "t", "tb", "p", "pb", "e", "eb", "ki", "kib", "mi", "mib", "gi", "gib",
        "ti", "tib", "pi", "pib", "ei", "eib",
    ];
    if text.parse::<u64>().is_ok() {
        return true;
    }
    let number_end = text.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(text.len());
    let (number, unit) = text.split_at(number_end);
    number.parse::<f64>().is_ok() && UNITS.iter().any(|known| unit.trim_start().eq_ignore_ascii_case(known))
}

/// A bare `def`/`extern` name that nu does not take for a name: one starting
/// with `-` (a lone `-`; the others are flags), for which nu takes the next
/// item that does not start with `-` (`def - [] {}` has "no space between name
/// and parameters", `extern - []` predeclares a command it then cannot find),
/// and a spread (`def ...{x: 1} [] {}`), which is no positional.
fn reject_name_item(working_set: &WorkingSet<'_>, token: &Token) -> ParseResult<()> {
    let text = working_set.get_span_contents(token.span);
    if text.starts_with('-') || is_spread(text, b"[{$(") {
        return Err(cut(Diagnostic::message("command name not supported", token.span)
            .with_help("a bare command name cannot start with `-` or be a spread; quote it")));
    }
    Ok(())
}

/// The name item of a `def`/`extern`: a string literal. Like nu, a name
/// containing `[` or `(` (even quoted) is "no space between name and
/// parameters", and a `$` item is not a string.
fn parse_def_name<'a>(working_set: &WorkingSet<'a>, token: &Token) -> ParseResult<Spanned<Cow<'a, str>>> {
    let text = working_set.get_span_contents(token.span);
    if let Some(at) = text.find(['[', '(']) {
        let span = Span::point(token.span.start + at);
        return Err(cut(Diagnostic::message("no space between name and parameters", span)
            .with_help("consider adding a space between the command's name and its parameters")));
    }
    if text.starts_with('$') {
        return Err(cut(Diagnostic::expected("string", token.span).with_help("the name of a definition is a string")));
    }
    parse_definition_name(working_set, token.span)
}

/// `def [--env] [--wrapped] name signature [: input/output types] { body }`;
/// nu also accepts the flags after the name.
pub fn parse_def<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let mut call = KeywordCall::start(&mut tokens)?;
    let mut flags = parse_def_flags(&mut tokens)?;
    let Some(name) = call.positional(&mut tokens, "command name")? else { return call.help_call() };
    reject_name_item(working_set, &name)?;
    let name = parse_def_name(working_set, &name)?;
    check_definition_name(&name, "command")?;
    flags.extend(parse_def_flags(&mut tokens)?);
    call.flags(&mut tokens)?;
    // The signature positional takes every remaining item but the last, which
    // is the body's: `def foo [] {} --help` drops the `{}` (two-item form of
    // `parse_full_signature`) and `def foo [] {} {} --help` has no colon.
    let (signature_items, body) = match tokens.remaining() {
        [] if call.wants_help() => return call.help_call(),
        [] => return Err(cut(Diagnostic::expected("signature", tokens.end_span()))),
        [only] => (std::slice::from_ref(only), None),
        [signature_items @ .., body] => (signature_items, Some(body)),
    };
    let signature = parse_full_signature(working_set, signature_items, false)?;
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
        Some(token) => parse_def_body(working_set, token, "definition body closure { ... }")?,
        None if call.wants_help() => return call.help_call(),
        None => return Err(cut(Diagnostic::expected("block", signature_items[0].span.past()))),
    };
    if flags.iter().any(|flag| flag.item == DefFlag::Wrapped) {
        check_wrapped_signature(&signature, name.span)?;
    }
    let span = call.keyword.span.merge(body.map_or(signature.span, |token| token.span));
    let def = Def { flags, name, signature, body_params, body: body_block };
    call.finish(Expression::new(Expr::Def(Box::new(def)), span))
}

/// The body of a `def`: nu parses it as a closure without looking at its
/// shape first (`def f [] {a: 1}` calls `a:`), and drops its parameters.
fn parse_def_body<'a>(
    working_set: &WorkingSet<'a>,
    token: &Token,
    what: &'static str,
) -> ParseResult<(Option<Signature<'a>>, Block<'a>)> {
    if token.contents != TokenContents::Item || !working_set.get_span_contents(token.span).starts_with('{') {
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

/// `def --wrapped` needs a rest parameter that is untyped or a `string`.
fn check_wrapped_signature(signature: &Signature<'_>, name_span: Span) -> ParseResult<()> {
    let rest = signature.params.iter().find(|p| matches!(p.kind, crate::ast::ParameterKind::Rest));
    let Some(rest) = rest else {
        return Err(cut(Diagnostic::message("missing required positional argument", name_span).with_help(
            "def --wrapped must have a ...rest-like positional argument; add `...rest: string` to the signature",
        )));
    };
    match &rest.ty {
        None => Ok(()),
        Some(ty) if ty.shape == crate::ast::SyntaxShape::String => Ok(()),
        Some(ty) => Err(cut(Diagnostic::message("type mismatch", ty.span).with_help(format!(
            "the ...rest-like positional argument of `def --wrapped` supports only strings; change the type of ...{} to `string`",
            rest.name.item
        )))),
    }
}

/// The `--env` and `--wrapped` flags of a `def`, up to a `--help`, a `--` or
/// an item that is not a long flag.
fn parse_def_flags(tokens: &mut Tokens<'_, '_>) -> ParseResult<Vec<Spanned<DefFlag>>> {
    repeat(0.., def_flag).parse_next(tokens)
}

/// One `--env` or `--wrapped`. Backtracks at a `--help`, a `--` or an item
/// that is not a long flag, which ends the `repeat` of [`parse_def_flags`]; any
/// other long flag is an error.
fn def_flag(tokens: &mut Tokens<'_, '_>) -> ParseResult<Spanned<DefFlag>> {
    let working_set = tokens.working_set;
    let flag = item
        .verify(|token| {
            let text = working_set.get_span_contents(token.span);
            text.starts_with("--") && !matches!(text, "--help" | "--")
        })
        .parse_next(tokens)?;
    match tokens.text(&flag) {
        "--env" => Ok(Spanned::new(DefFlag::Env, flag.span)),
        "--wrapped" => Ok(Spanned::new(DefFlag::Wrapped, flag.span)),
        other => Err(cut(Diagnostic::message(format!("the `def` command doesn't have flag `{other}`"), flag.span)
            .with_help("`def` accepts `--env` and `--wrapped`"))),
    }
}

/// `extern name signature [: input/output types]`.
pub fn parse_extern<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let mut call = KeywordCall::start(&mut tokens)?;
    let Some(name) = call.positional(&mut tokens, "command name")? else { return call.help_call() };
    reject_name_item(working_set, &name)?;
    let name = parse_def_name(working_set, &name)?;
    check_definition_name(&name, "command")?;
    call.flags(&mut tokens)?;
    let rest = tokens.remaining();
    let Some(last) = rest.last() else {
        if call.wants_help() {
            return call.help_call();
        }
        return Err(cut(Diagnostic::expected("signature", tokens.end_span())));
    };
    // The signature argument takes every remaining item, so a body after it
    // (the old `extern-wrapped`) is dropped by nu without a look.
    let signature = parse_full_signature(working_set, rest, true)?;
    let span = call.keyword.span.merge(last.span);
    call.finish(Expression::new(Expr::Extern(Box::new(Extern { name, signature })), span))
}

/// `for variable[: type] in iterable { block }`.
pub fn parse_for<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let mut call = KeywordCall::start(&mut tokens)?;
    let Some(variable) = call.positional(&mut tokens, "loop variable")? else { return call.help_call() };
    let (var, typed) = parse_var_with_opt_type(working_set, &variable)?;
    // The variable positional runs up to the `in` keyword, so a type after
    // `x:` is every item before it (`record<a: int, b: string>` is four items)
    // and `for x: int --help in [] {}` has the unknown type `int --help`.
    let ty = match typed {
        true => {
            let Some(type_span) = tokens_until("in").parse_next(&mut tokens)?.span() else {
                return Err(cut(Diagnostic::expected("type", variable.span.past())));
            };
            Some(parse_var_type(working_set, type_span)?)
        }
        false => None,
    };
    let Some(in_keyword) = call.positional(&mut tokens, "`in`")? else { return call.help_call() };
    if tokens.text(&in_keyword) != "in" {
        return Err(cut(Diagnostic::new(ErrorKind::ExpectedKeyword("in"), in_keyword.span)));
    }
    // nu reserves the last item for the block before it parses the keyword's
    // argument, so `for x in []` and `for x --help in []` lack the argument of
    // `in` (KeywordMissingArgument), help or not.
    if tokens.remaining().len() < 2 {
        return Err(cut(Diagnostic::message("missing argument to `in`", in_keyword.span)
            .with_help("`for` needs a value to iterate and a block: `for x in [1 2] { }`")));
    }
    let iterable = tokens.expect_item("value to iterate")?;
    let iterable = Box::new(parse_value(working_set, iterable.span, ExpectedShape::Any)?);
    let Some(block) = call.positional(&mut tokens, "block")? else { return call.help_call() };
    let (body, body_value) = parse_block_or_value_argument(working_set, &block, "block")?;
    call.end(&mut tokens)?;
    let span = call.keyword.span.merge(block.span);
    let for_loop = For { var, ty, in_keyword: in_keyword.span, iterable, body, body_value };
    call.finish(Expression::new(Expr::For(Box::new(for_loop)), span))
}
