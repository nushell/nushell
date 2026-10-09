//! `module`, `use`, `export` and `export-env` (nu-parser's `parse_module.rs`).

use crate::ast::{
    Export, ExportEnv, Expr, Expression, ImportPatternMember, ImportPatternMemberKind, ListItem, Module, Quote,
    RecordItem, SyntaxShape, Use,
};
use crate::error::{Diagnostic, ErrorKind};
use crate::input::{ParseResult, cut};
use crate::lex::{Token, TokenContents};
use crate::span::{Span, Spanned};

use super::WorkingSet;
use super::parse_alias::parse_alias;
use super::parse_expressions::{
    ExpectedShape, parse_block_body_unchecked, parse_builtin_commands, parse_list_expression, parse_value,
};
use super::parse_helpers::is_help_flag;
use super::parse_keywords::{
    KeywordCall, keyword_boundary, keyword_boundary_with, parse_block_argument, parse_help_call,
};
use super::parse_literals::parse_full_cell_path;
use super::tokens::Tokens;

/// The name of a `module`: a string literal, quoted or not. nu's `module`
/// takes a `string` argument and then needs a literal: a variable, a
/// subexpression or an interpolation is not one.
fn parse_module_name<'a>(working_set: &WorkingSet<'a>, token: &Token) -> ParseResult<Expression<'a>> {
    let text = working_set.get_span_contents(token.span);
    if text.starts_with(['$', '(', '{']) {
        return Err(
            cut(Diagnostic::expected("string", token.span).with_help("the name of a module must be a literal")),
        );
    }
    let name = parse_value(working_set, token.span, ExpectedShape::String)?;
    match name.expr {
        Expr::String(_) => Ok(name),
        _ => Err(cut(Diagnostic::expected("string", token.span).with_help("the name of a module must be a literal"))),
    }
}

/// `module name [{ body }]` (nu's `parse_module`): a module declared in the
/// file, or a module file to load. nu takes the arguments by position, so a
/// `--` is not an end-of-options marker.
pub fn parse_module<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let statement = tokens;
    if let Some(call) = parse_module_help_call(statement) {
        return call;
    }
    let keyword = tokens.expect_item("module")?;
    // A `--help` here follows a `--` (`module -- --help`), so it is a name.
    keyword_boundary_with(&mut tokens, "module", &[], false)?;
    let name_token = tokens.expect_item("module name or path")?;
    // nu first parses `module` as a call to find `--help`, and that parse
    // consumes a `--`: the item after it must then be a name (`module -- {}`
    // is a type mismatch, `module --` lacks its name). The statement itself
    // then takes `--` as the name.
    if working_set.get_span_contents(name_token.span) == "--" {
        match tokens.peek_token() {
            None => return Err(cut(Diagnostic::expected("module name after `--`", name_token.span.past()))),
            Some(token) if working_set.get_span_contents(token.span).starts_with(['{', '$', '(']) => {
                return Err(cut(
                    Diagnostic::expected("string", token.span).with_help("the name of a module must be a literal")
                ));
            }
            Some(_) => {}
        }
    }
    let name = parse_module_name(working_set, &name_token)?;
    keyword_boundary_with(&mut tokens, "module", &[], false)?;
    let mut end = name_token.span;
    let body = match tokens.peek_token() {
        Some(token)
            if token.contents == TokenContents::Item && working_set.get_span_contents(token.span).starts_with('{') =>
        {
            end = token.span;
            tokens.next_token();
            working_set.enter_scope();
            let body = parse_block_body_unchecked(working_set, token.span);
            working_set.exit_scope();
            let body = body?;
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
                // exports the alias it defined, so there it needs a target: `export
                // alias x =`, `export alias h home` and `export alias --help` fail.
                if let Expr::Export(export) = &first.expr
                    && is_alias_without_target(&export.item)
                {
                    return Err(cut(Diagnostic::message("incomplete alias", first.span)
                        .with_help("an alias needs a name, `=` and a command: `export alias x = ls`")));
                }
            }
            Some(body)
        }
        Some(token) => return Err(cut(Diagnostic::expected("block", token.span))),
        None => None,
    };
    keyword_boundary_with(&mut tokens, "module", &[], false)?;
    // nu reads a `--` after the body as the end of the options and drops it
    // (`module x {} --`); anything after it is an extra positional.
    if let Some(&marker) = tokens.peek_token()
        && body.is_some()
        && working_set.get_span_contents(marker.span) == "--"
    {
        working_set.add_ignored(marker.span);
        tokens.next_token();
    }
    tokens.expect_end()?;
    Ok(Expression::new(Expr::Module(Module { name: Box::new(name), body }), keyword.span.merge(end)))
}

/// `module ... --help`. nu parses `module` as a call before anything else,
/// looking for flags up to a `--` and taking only the name by its shape
/// (`ArgumentParsingLevel::FirstK`). When that call asks for help it is the
/// statement: the name is a `string`, the item in the block's place is never
/// parsed (`module x {ls} --help`, `module x --help {}#`), and a third item is
/// one too many.
fn parse_module_help_call<'a>(statement: Tokens<'_, 'a>) -> Option<ParseResult<Expression<'a>>> {
    let working_set = statement.working_set;
    let mut help = false;
    let mut end_of_options = false;
    let mut positionals = Vec::new();
    let mut unknown_flag = None;
    for token in statement.remaining().iter().skip(1).filter(|token| token.contents == TokenContents::Item) {
        match working_set.get_span_contents(token.span) {
            "--" if !end_of_options => end_of_options = true,
            "--help" | "-h" if !end_of_options => help = true,
            flag if !end_of_options && flag.len() > 1 && flag.starts_with('-') => {
                unknown_flag = unknown_flag.or(Some((flag, token.span)));
            }
            _ => positionals.push(token),
        }
    }
    if !help {
        return None;
    }
    if let Some((flag, span)) = unknown_flag {
        return Some(Err(cut(Diagnostic::message(format!("the `module` command doesn't have flag `{flag}`"), span)
            .with_help("use `--help` to see available flags"))));
    }
    if let Some(name) = positionals.first() {
        match parse_value(working_set, name.span, ExpectedShape::Declared(&SyntaxShape::String)) {
            Err(error) => return Some(Err(error)),
            // nu type-checks the name, and a record is not a string.
            Ok(Expression { expr: Expr::Record(_), .. }) => {
                return Some(Err(cut(Diagnostic::expected("string", name.span).with_help("found a record"))));
            }
            Ok(_) => {}
        }
    }
    if let Some(extra) = positionals.get(2) {
        return Some(Err(cut(
            Diagnostic::message("extra positional argument", extra.span).with_help("`module` takes a name and a block")
        )));
    }
    let mut tokens = statement;
    Some(match (KeywordCall::start_positional(&mut tokens), positionals.get(1)) {
        (Ok(call), Some(block)) => call.help_call_without(block),
        _ => parse_help_call(statement),
    })
}

/// An `alias` that defines nothing: no target after its `=`, or kept as a
/// plain call (no `=` at all, or a help call, which nu cannot export from a
/// module either).
fn is_alias_without_target(expr: &Expression<'_>) -> bool {
    match &expr.expr {
        Expr::Alias(alias) => alias.value.is_none(),
        Expr::Call(call) => call.head.name == "alias",
        _ => false,
    }
}

/// Whether `expr` may appear in a module body: declarations only.
fn is_module_item(expr: &Expression<'_>) -> bool {
    match &expr.expr {
        Expr::Def(_)
        | Expr::Extern(_)
        | Expr::Alias(_)
        | Expr::Use(_)
        | Expr::Module(_)
        | Expr::Export(_)
        | Expr::ExportEnv(_)
        | Expr::Const(_) => true,
        Expr::AttributeBlock(attribute_block) => is_module_item(&attribute_block.item),
        _ => false,
    }
}

/// `use module [members...]` (nu's `parse_use`, whose members are nu's import
/// pattern).
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
        if token.contents == TokenContents::Item && text.starts_with('-') && text.len() > 1 && text != "--" {
            return Err(cut(Diagnostic::message(format!("the `use` command doesn't have flag `{text}`"), token.span)
                .with_help("use `--help` to see available flags")));
        }
    }
    keyword_boundary(&mut tokens, "use", &[])?;
    let module_token = tokens.expect_item("module name or path")?;
    let module = match working_set.get_span_contents(module_token.span) {
        "null" => Expression::new(Expr::Nothing, module_token.span),
        _ => {
            let module = parse_value(working_set, module_token.span, ExpectedShape::String)?;
            if let Expr::Record(_) = module.expr {
                return Err(cut(Diagnostic::expected("string", module_token.span).with_help("found a record")));
            }
            module
        }
    };
    // After `use null` nu parses the members and then never looks at them.
    let noop = matches!(module.expr, Expr::Nothing);
    let mut members = Vec::new();
    let mut end = module_token.span;
    while let Some(token) = tokens.next_token() {
        if working_set.get_span_contents(token.span) == "--" {
            working_set.add_ignored(token.span);
            continue;
        }
        if members.last().is_some_and(|member: &ImportPatternMember<'_>| {
            matches!(member.kind, ImportPatternMemberKind::Glob | ImportPatternMemberKind::List(_))
        }) {
            return Err(cut(Diagnostic::message(
                "a `*` or `[...]` member can only be at the end of an import pattern",
                token.span,
            )));
        }
        members.push(parse_import_pattern_member(working_set, token, noop)?);
        end = token.span;
    }
    Ok(Expression::new(Expr::Use(Use { module: Box::new(module), members }), keyword.span.merge(end)))
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
        cut(Diagnostic::message("wrong import pattern structure", expr.span)
            .with_help("only strings and lists of strings can be imported"))
    };
    let kind = match text.as_bytes()[0] {
        b'*' if text == "*" => ImportPatternMemberKind::Glob,
        b'[' => return parse_import_pattern_list(working_set, token),
        b'$' | b'(' | b'{' => {
            let expr = parse_value(working_set, span, ExpectedShape::Any)?;
            match &expr.expr {
                Expr::Var(_) | Expr::FullCellPath(_) | Expr::Subexpression(_) => {}
                Expr::Record(items) if matches!(items.first(), Some(RecordItem::Pair { .. })) => {}
                _ if noop => {}
                _ => return Err(wrong(&expr)),
            }
            ImportPatternMemberKind::Ignored(Box::new(expr))
        }
        _ => {
            let expr = parse_value(working_set, span, ExpectedShape::Any)?;
            match expr.expr {
                Expr::String(string) if !matches!(string.quote, Quote::Raw(_)) => {
                    ImportPatternMemberKind::Name(string.value)
                }
                _ if noop => ImportPatternMemberKind::Ignored(Box::new(expr)),
                _ => return Err(wrong(&expr)),
            }
        }
    };
    Ok(ImportPatternMember { span, kind })
}

/// The names in a `use module [a b c]` list. Items that are not strings are
/// parsed and ignored, as is a cell path after the list (`[math].x`).
fn parse_import_pattern_list<'a>(working_set: &WorkingSet<'a>, token: &Token) -> ParseResult<ImportPatternMember<'a>> {
    let span = token.span;
    let list_end = working_set.group_end(span).map_or(span.end, |close| span.start + close + 1);
    let list = parse_list_expression(working_set, Span::new(span.start, list_end))?;
    if list_end < span.end {
        // Parse the cell path for its errors, then drop it like nu.
        parse_full_cell_path(working_set, span, false)?;
        working_set.add_ignored(Span::new(list_end, span.end));
    }
    let Expr::List(items) = list.expr else {
        return Err(cut(Diagnostic::expected("list of names", span)));
    };
    let mut names = Vec::new();
    for item in items {
        match item {
            ListItem::Item(Expression { span, expr: Expr::String(string) }) => {
                names.push(Spanned::new(string.value, span))
            }
            ListItem::Item(other) => working_set.add_ignored(other.span),
            ListItem::Spread { dots, .. } => {
                return Err(cut(Diagnostic::message("cannot spread in an import pattern", dots)));
            }
        }
    }
    Ok(ImportPatternMember { span, kind: ImportPatternMemberKind::List(names) })
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
                "`def`, `extern`, `alias`, `use`, `module` or `const` after `export`",
                keyword.span.past(),
            )));
        }
    };
    match working_set.get_span_contents(next.span) {
        "def" | "extern" | "alias" | "use" | "module" | "const" => {}
        "--help" | "-h" => {
            // `export` has no positionals and no flag but help, so `export --help
            // -h` shows help and `export --help x` is one positional too many.
            if let Some(extra) = items[2..].iter().find(|token| !is_help_flag(working_set, token)) {
                return Err(cut(Diagnostic::new(ErrorKind::ExtraTokens, extra.span)
                    .with_help("`export` takes no positional arguments")));
            }
            return parse_help_call(tokens);
        }
        other => {
            return Err(cut(Diagnostic::message(format!("`export {other}` is not a valid export"), next.span)
                .with_help("expected `def`, `extern`, `alias`, `use`, `module` or `const`")));
        }
    }
    let rest = tokens.slice(1..items.len());
    let item = match working_set.get_span_contents(next.span) {
        "alias" => parse_alias(rest, true).map_err(|error| error.map(|failure| failure.with_context("alias")))?,
        _ => parse_builtin_commands(rest)?,
    };
    let span = keyword.span.merge(item.span);
    Ok(Expression::new(Expr::Export(Export { item: Box::new(item) }), span))
}

/// `export-env { block }`. nu takes its one argument by position.
pub fn parse_export_env<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let mut call = KeywordCall::start_positional(&mut tokens)?;
    let Some(block) = call.positional(&mut tokens, "block")? else { return call.help_call() };
    let body = parse_block_argument(working_set, &block, "block")?;
    // nu hands `export-env` exactly one argument: anything after it is dropped.
    for ignored in tokens.remaining() {
        working_set.add_ignored(ignored.span);
    }
    let span = call.keyword.span.merge(block.span);
    call.finish(Expression::new(Expr::ExportEnv(ExportEnv { body }), span))
}
