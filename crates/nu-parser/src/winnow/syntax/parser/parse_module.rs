//! `module`, `use`, `export` and `export-env` (nu-parser's `parse_module.rs`).

use crate::winnow::syntax::ast::{
    Argument, Export, ExportEnv, Expr, Expression, ImportPatternMember, ImportPatternMemberKind,
    ListItem, Module, Quote, RecordItem, Use,
};
use crate::winnow::syntax::error::{Diagnostic, ErrorKind};
use crate::winnow::syntax::input::{ParseResult, cut};
use crate::winnow::syntax::lex::{Token, TokenContents};
use crate::winnow::syntax::span::{Span, Spanned};

use super::WorkingSet;
use super::parse_expressions::{
    ExpectedShape, parse_block_body_unchecked, parse_builtin_commands, parse_list_expression,
    parse_value,
};
use super::parse_helpers::{is_help_flag, is_help_text, is_spread};
use super::parse_keywords::{KeywordCall, keyword_boundary, parse_block_argument, parse_help_call};
use super::parse_literals::parse_full_cell_path;
use super::parse_type_alias::parse_type_alias;
use super::tokens::Tokens;

/// The name of a `module`: a string literal, quoted or not. nu's `module`
/// takes a `string` argument and then needs a literal: a variable, a
/// subexpression or an interpolation is not one.
fn parse_module_name<'a>(
    working_set: &WorkingSet<'a>,
    token: &Token,
) -> ParseResult<Expression<'a>> {
    let text = working_set.get_span_contents(token.span);
    if text.starts_with(['$', '(', '{']) {
        return Err(cut(Diagnostic::expected("string", token.span)
            .with_help("the name of a module must be a literal")));
    }
    let name = parse_value(working_set, token.span, ExpectedShape::String)?;
    match name.expr {
        Expr::String(_) => Ok(name),
        _ => Err(cut(Diagnostic::expected("string", token.span)
            .with_help("the name of a module must be a literal"))),
    }
}

/// `module name [{ body }]` (nu's `parse_module`): a module declared in the
/// file, or a module file to load. nu first reads the statement as a call to
/// `module` ([`parse_module_call`]) and then takes the item after the name as
/// the body. The statement is a declaration when it is only that: a literal
/// name, then a block or nothing, then a `--` after the block, which nu drops.
/// Anything else (a call for help, a flag, a `--` elsewhere, a name that is no
/// literal, an item in the body's place that is no block, one item too many)
/// is that call, whose errors nu reports as a call's: the consumer's.
pub fn parse_module<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let statement = tokens;
    let keyword = tokens.expect_item("module")?;
    let items: Vec<Token> = tokens
        .remaining()
        .iter()
        .filter(|token| token.contents == TokenContents::Item)
        .copied()
        .collect();
    let text = |token: &Token| working_set.get_span_contents(token.span);
    let declaration = match items.as_slice() {
        [name] => Some((name, None)),
        [name, body] => Some((name, Some(body))),
        [name, body, marker] if text(marker) == "--" => Some((name, Some(body))),
        _ => None,
    };
    let Some((name_token, body_token)) = declaration.filter(|(name, body)| {
        !is_flag(text(name))
            && !is_spread(text(name), b"[$({")
            && body.is_none_or(|body| text(body).starts_with('{'))
    }) else {
        return parse_module_call(statement);
    };
    let Ok(name) = parse_module_name(working_set, name_token) else {
        return parse_module_call(statement);
    };
    let mut end = name_token.span;
    let body = match body_token {
        Some(token) => {
            end = token.span;
            let body = parse_block_body_unchecked(working_set, token.span)?;
            // Like nu, a module body holds declarations only.
            for pipeline in &body.pipelines {
                let first = &pipeline.elements[0].expr;
                if !is_module_item(first) {
                    return Err(cut(Diagnostic::new(
                        ErrorKind::ExpectedKeyword("def, const, extern, alias, use, module, export or export-env"),
                        first.span,
                    )
                    .with_help("a module body can only declare things; put code in `export-env` or a `def`")));
                }
                // nu parses a module's `export alias` without the `export` and then
                // exports the alias it defined, so a call for help (`export alias
                // --help`), which defines none, fails there.
                if let Expr::Export(export) = &first.expr
                    && is_alias_help_call(&export.item)
                {
                    return Err(cut(Diagnostic::message("incomplete alias", first.span)
                        .with_help(
                            "an alias needs a name, `=` and a command: `export alias x = ls`",
                        )));
                }
            }
            Some(body)
        }
        None => None,
    };
    // nu reads a `--` after the body as the end of the options and drops it (`module x {} --`).
    if let Some(marker) = items.get(2) {
        working_set.add_ignored(marker.span);
    }
    Ok(Expression::new(
        Expr::Module(Module {
            name: Box::new(name),
            body,
        }),
        keyword.span.merge(end),
    ))
}

/// Whether the item `text` is a flag where nu's call looks for one, before a `--`: any item
/// of more than one character starting with `-` (`--foo`, `-h`, `-1`, `--`).
fn is_flag(text: &str) -> bool {
    text.len() > 1 && text.starts_with('-')
}

/// The call nu makes to `module` before anything else (`parse_module`, which reads it with
/// `ArgumentParsingLevel::FirstK { k: 1 }`): flags up to a `--`, the first positional, and every
/// later positional left unparsed (`Argument::Unread`: `module x --help {ls}` parses). nu gives this
/// call back as the statement when it asks for help or misses its name; otherwise it goes on
/// with the item after the name as the body, whatever that item is (`module x --foo {}` has
/// `--foo` there).
fn parse_module_call<'a>(statement: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = statement.working_set;
    let mut end_of_options = false;
    let mut name_seen = false;
    let mut unparsed = Vec::new();
    for token in statement
        .remaining()
        .iter()
        .skip(1)
        .filter(|token| token.contents == TokenContents::Item)
    {
        let text = working_set.get_span_contents(token.span);
        if !end_of_options && text == "--" {
            end_of_options = true;
        } else if !end_of_options && is_flag(text) || is_spread(text, b"[$({") {
            // A flag, or a spread, which nu reads before any positional (`module x ...$l`).
        } else if !name_seen {
            name_seen = true;
        } else {
            unparsed.push(*token);
        }
    }
    let items: Vec<Token> = statement
        .remaining()
        .iter()
        .filter(|token| !unparsed.iter().any(|item| item.span == token.span))
        .copied()
        .collect();
    let mut call = parse_help_call(Tokens::new(working_set, &items, statement.end_span().start))?;
    if let Expr::Call(call) = &mut call.expr {
        call.arguments
            .extend(unparsed.iter().map(|token| Argument::Unread(token.span)));
        call.arguments.sort_by_key(Argument::span);
    }
    if let Some(span) = statement.span() {
        call.span = span;
    }
    Ok(call)
}

/// An `alias` kept as a plain call: a call for help, which defines no alias.
fn is_alias_help_call(expr: &Expression<'_>) -> bool {
    matches!(&expr.expr, Expr::Call(call) if call.head.name == "alias")
}

/// The statements nu's `parse_module_block` parses in a module body, by their first word; any
/// other is an error there.
const MODULE_KEYWORDS: &[&str] = &[
    "def",
    "extern",
    "alias",
    "type",
    "use",
    "module",
    "const",
    "export-env",
];

/// Whether `expr` may appear in a module body: declarations only, or one of their keywords
/// parsed as a call (`def --help`), which nu's parse function for the keyword gives back.
fn is_module_item(expr: &Expression<'_>) -> bool {
    match &expr.expr {
        Expr::Def(_)
        | Expr::Extern(_)
        | Expr::Alias(_)
        | Expr::TypeAlias(_)
        | Expr::Use(_)
        | Expr::Module(_)
        | Expr::Export(_)
        | Expr::ExportEnv(_)
        | Expr::Const(_) => true,
        Expr::Call(call) => MODULE_KEYWORDS.contains(&call.head.name.as_ref()),
        Expr::AttributeBlock(attribute_block) => is_module_item(&attribute_block.item),
        _ => false,
    }
}

/// `use module [members...]` (nu's `parse_use`, whose members are nu's import
/// pattern).
///
/// ```text
/// use    = "use" module { member }
/// module = string | "null"
/// member = name | "*" | "[" { name } "]" | $var | (expr) | { key: value }
/// ```
///
/// A `*` or a list ends the pattern; the last three members are parsed and
/// ignored ([`parse_import_pattern_member`]).
pub fn parse_use<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let statement = tokens;
    let keyword = tokens.expect_item("use")?;
    // Flags are looked for before every argument of `use`.
    for token in tokens.remaining() {
        if is_help_flag(working_set, token) {
            return parse_help_call(statement);
        }
        let text = working_set.get_span_contents(token.span);
        if token.contents == TokenContents::Item
            && text.starts_with('-')
            && text.len() > 1
            && text != "--"
        {
            return Err(cut(Diagnostic::message(
                format!("the `use` command doesn't have flag `{text}`"),
                token.span,
            )
            .with_help("use `--help` to see available flags")));
        }
    }
    // nu drops the first `--`, which ends the options; a later one is an argument.
    let mut end_of_options = tokens
        .peek_token()
        .filter(|token| working_set.get_span_contents(token.span) == "--")
        .map(|token| token.span);
    keyword_boundary(&mut tokens, "use", &[])?;
    let module_token = tokens.expect_item("module name or path")?;
    let module = match working_set.get_span_contents(module_token.span) {
        "null" => Expression::new(Expr::Nothing, module_token.span),
        _ => {
            let module = parse_value(working_set, module_token.span, ExpectedShape::String)?;
            if let Expr::Record(_) = module.expr {
                return Err(cut(
                    Diagnostic::expected("string", module_token.span).with_help("found a record")
                ));
            }
            module
        }
    };
    // After `use null` nu parses the members and then never looks at them.
    let noop = matches!(module.expr, Expr::Nothing);
    let mut members = Vec::new();
    let mut end = module_token.span;
    while let Some(token) = tokens.next_token() {
        if end_of_options.is_none() && working_set.get_span_contents(token.span) == "--" {
            working_set.add_ignored(token.span);
            end_of_options = Some(token.span);
            continue;
        }
        if members
            .last()
            .is_some_and(|member: &ImportPatternMember<'_>| {
                matches!(
                    member.kind,
                    ImportPatternMemberKind::Glob | ImportPatternMemberKind::List(_)
                )
            })
        {
            return Err(cut(Diagnostic::message(
                "a `*` or `[...]` member can only be at the end of an import pattern",
                token.span,
            )));
        }
        members.push(parse_import_pattern_member(working_set, token, noop)?);
        end = token.span;
    }
    Ok(Expression::new(
        Expr::Use(Use {
            module: Box::new(module),
            members,
            end_of_options,
        }),
        keyword.span.merge(end),
    ))
}

/// One member of an import pattern, classified as nu does from the parsed
/// value: a string is a name (`*` the glob), a list gives its string items,
/// a variable, subexpression or `key: value` record is parsed and ignored,
/// and anything else is a "wrong import pattern" unless `noop` (after
/// `use null`) makes every member irrelevant. A raw string is not a name
/// (nu's `Expr::RawString`). `hide` classifies its members the same way.
pub fn parse_import_pattern_member<'a>(
    working_set: &WorkingSet<'a>,
    token: &Token,
    noop: bool,
) -> ParseResult<ImportPatternMember<'a>> {
    let text = working_set.get_span_contents(token.span);
    let span = token.span;
    let wrong = |expr: &Expression<'_>| {
        cut(
            Diagnostic::message("wrong import pattern structure", expr.span)
                .with_help("only strings and lists of strings can be imported"),
        )
    };
    if text.starts_with('[') {
        return parse_import_pattern_list(working_set, token);
    }
    let value = parse_value(working_set, span, ExpectedShape::Any)?;
    let kind = if text == "*" {
        ImportPatternMemberKind::Glob
    } else if text.starts_with(['$', '(', '{']) {
        match &value.expr {
            Expr::Var(_) | Expr::FullCellPath(_) | Expr::Subexpression(_) => {}
            Expr::Record(items) if matches!(items.first(), Some(RecordItem::Pair { .. })) => {}
            _ if noop => {}
            _ => return Err(wrong(&value)),
        }
        ImportPatternMemberKind::Ignored
    } else {
        match &value.expr {
            Expr::String(string) if !matches!(string.quote, Quote::Raw(_)) => {
                ImportPatternMemberKind::Name(string.value.clone())
            }
            _ if noop => ImportPatternMemberKind::Ignored,
            _ => return Err(wrong(&value)),
        }
    };
    Ok(ImportPatternMember { span, kind, value })
}

/// The names in a `use module [a b c]` list. Items that are not strings are
/// parsed and ignored, as is a cell path after the list (`[math].x`), which
/// the member's value keeps.
fn parse_import_pattern_list<'a>(
    working_set: &WorkingSet<'a>,
    token: &Token,
) -> ParseResult<ImportPatternMember<'a>> {
    let span = token.span;
    let list_end = working_set
        .group_end(span)
        .map_or(span.end, |close| span.start + close + 1);
    let list = parse_list_expression(working_set, Span::new(span.start, list_end))?;
    let cell_path = if list_end < span.end {
        let cell_path = parse_full_cell_path(working_set, span, false)?;
        working_set.add_ignored(Span::new(list_end, span.end));
        Some(cell_path)
    } else {
        None
    };
    let Expr::List(items) = &list.expr else {
        return Err(cut(Diagnostic::expected("list of names", span)));
    };
    let mut names = Vec::new();
    for item in items {
        match item {
            ListItem::Item(Expression {
                span,
                expr: Expr::String(string),
            }) => names.push(Spanned::new(string.value.clone(), *span)),
            ListItem::Item(other) => working_set.add_ignored(other.span),
            ListItem::Spread { dots, .. } => {
                return Err(cut(Diagnostic::message(
                    "cannot spread in an import pattern",
                    *dots,
                )));
            }
        }
    }
    Ok(ImportPatternMember {
        span,
        kind: ImportPatternMemberKind::List(names),
        value: cell_path.unwrap_or(list),
    })
}

/// `export def|extern|alias|use|module|const ...` (nu's `parse_export_in_block`).
pub fn parse_export_in_block<'a>(tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let items = tokens.all();
    let keyword = items[0];
    let next = match items.get(1) {
        Some(token) if token.contents == TokenContents::Item => token,
        _ => {
            return Err(cut(Diagnostic::expected(
                "`def`, `extern`, `alias`, `type`, `use`, `module` or `const` after `export`",
                keyword.span.past(),
            )));
        }
    };
    match working_set.get_span_contents(next.span) {
        "def" | "extern" | "alias" | "type" | "use" | "module" | "const" => {}
        text if is_help_text(text) => {
            // `export` has no positionals and no flag but help, so `export --help
            // -h` shows help and `export --help x` is one positional too many.
            if let Some(extra) = items[2..]
                .iter()
                .find(|token| !is_help_flag(working_set, token))
            {
                return Err(cut(Diagnostic::new(ErrorKind::ExtraTokens, extra.span)
                    .with_help("`export` takes no positional arguments")));
            }
            return parse_help_call(tokens);
        }
        other => {
            return Err(cut(Diagnostic::message(
                format!("`export {other}` is not a valid export"),
                next.span,
            )
            .with_help(
                "expected `def`, `extern`, `alias`, `type`, `use`, `module` or `const`",
            )));
        }
    }
    let rest = tokens.slice(1..items.len());
    let item = match working_set.get_span_contents(next.span) {
        "type" => parse_type_alias(rest, true)
            .map_err(|error| error.map(|failure| failure.with_context("type")))?,
        _ => parse_builtin_commands(rest)?,
    };
    let span = keyword.span.merge(item.span);
    Ok(Expression::new(
        Expr::Export(Export {
            item: Box::new(item),
        }),
        span,
    ))
}

/// `export-env { block }`. nu takes its one argument by position.
pub fn parse_export_env<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let statement = tokens;
    let mut call = KeywordCall::start_positional(&mut tokens)?;
    // nu hands `export-env` exactly one argument, a help flag too: anything after it is
    // dropped (`export-env --help x`).
    if call.help_flag(&mut tokens)
        && let [keyword, help, dropped @ ..] = statement.remaining()
    {
        let help_call =
            parse_help_call(Tokens::new(working_set, &[*keyword, *help], help.span.end))?;
        for ignored in dropped {
            working_set.add_ignored(ignored.span);
        }
        return Ok(help_call);
    }
    let Some(block) = call.positional(&mut tokens, "block")? else {
        return call.help_call();
    };
    let body = parse_block_argument(working_set, &block, "block")?;
    // nu hands `export-env` exactly one argument: anything after it is dropped.
    for ignored in tokens.remaining() {
        working_set.add_ignored(ignored.span);
    }
    let span = call.keyword.span.merge(block.span);
    call.finish(Expression::new(Expr::ExportEnv(ExportEnv { body }), span))
}
