//! Expressions: values, collections, blocks and closures, math, assignments
//! and the dispatch of builtin commands (nu-parser's `parse_expressions.rs`).

use winnow::Parser;
use winnow::combinator::{Infix, eof, expression, opt, preceded, repeat, repeat_till};
use winnow::error::ErrMode;
use winnow::stream::Location;

use crate::ast::{
    Assignment, BinaryOp, Block, Closure, EnvAssignment, EnvShorthand, Expr, Expression, InterpolationPart, ListItem,
    MatchArm, MatchPattern, Operator, Pattern, Quote, RecordItem, Signature, StringLiteral, SyntaxShape, Table,
    TypeAnnotation, UnaryNot,
};
use crate::error::{Diagnostic, ErrorKind};
use crate::input::{Input, ParseFailure, ParseResult, backtrack, cut, input};
use crate::lex::{LexOptions, Token, TokenContents, assignment_operator};
use crate::span::{Span, Spanned};

use super::WorkingSet;
use super::lite_parser::lite_parse_parts;
use super::parse_alias::parse_alias;
use super::parse_bindings::{parse_const, parse_let, parse_mut};
use super::parse_calls::{find_longest_decl, parse_call, parse_external_string};
use super::parse_control_flow::{
    parse_break_or_continue, parse_if, parse_loop, parse_match, parse_return, parse_try, parse_while,
};
use super::parse_def::{parse_def, parse_extern, parse_for};
use super::parse_helpers::{delimited_interior, invalid_literal, is_spread};
use super::parse_keywords::is_statement_keyword;
use super::parse_literals::{
    is_datetime, is_range_head, is_range_syntax, looks_like_binary, parse_binary, parse_dollar_expr, parse_duration,
    parse_filesize, parse_float, parse_full_cell_path, parse_int, parse_number, parse_paren_expr, parse_range,
    parse_raw_string, parse_simple_cell_path, parse_string, parse_string_literal, radix_prefix,
};
use super::parse_module::{parse_export_env, parse_export_in_block, parse_module, parse_use};
use super::parse_patterns::pattern;
use super::parse_pipelines::parse_block;
use super::parse_signatures::parse_signature_helper;
use super::parse_source::parse_where;
use super::tokens::{Tokens, expected, item, keyword, pipe, tokens_until};

/// Where a command sits, which decides how its head is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    /// The only command of its pipeline: statement keywords (`def`, `let`,
    /// `use`, ...) are statements (nu's `parse_builtin_commands`).
    Statement,
    /// One element of a longer pipeline, the left side of an assignment, an
    /// `else` or match-arm expression: nu's `parse_expression`, where a
    /// declaration is an error.
    Element,
}

/// The items of one pipeline element (nu's `parse_expression`).
///
/// Handles, in this order: statement keywords (in statement position, nu's
/// `parse_builtin_commands`), `NAME=value` environment shorthand,
/// assignments, math expressions (when the first item looks like a value),
/// and finally keyword expressions and calls.
pub fn parse_expression<'a>(mut tokens: Tokens<'_, 'a>, position: Position) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let Some(first) = tokens.peek_token() else {
        return Err(cut(Diagnostic::expected("command", tokens.end_span())));
    };
    if position == Position::Statement
        && first.contents == TokenContents::Item
        && is_statement_keyword(working_set.get_span_contents(first.span))
    {
        return parse_builtin_commands(tokens);
    }
    let env_assignments = parse_env_shorthand_prefix(&mut tokens)?;
    let Some(first) = tokens.peek_token() else {
        return Err(cut(Diagnostic::message("unknown command", tokens.all()[0].span)
            .with_help("`NAME=value` sets an environment variable for the command that follows it")));
    };
    // After environment shorthand nu reads the head like any pipeline element.
    if first.contents == TokenContents::Item && (!env_assignments.is_empty() || position == Position::Element) {
        check_builtin_command_in_pipeline(&tokens)?;
    }
    let expression =
        if tokens.remaining().iter().any(|token| matches!(token.contents, TokenContents::AssignmentOperator(_))) {
            parse_assignment_expression(tokens.rest_stream())?
        } else if first.contents != TokenContents::Item {
            return Err(cut(Diagnostic::expected("command", first.span)));
        } else if is_math_expression_like(working_set.get_span_contents(first.span)) {
            parse_math_expression(tokens.rest_stream())?
        } else {
            parse_builtin_commands(tokens.rest_stream())?
        };
    match env_assignments.first() {
        None => Ok(expression),
        Some(first) => {
            let span = first.span.merge(expression.span);
            let env_shorthand = EnvShorthand { vars: env_assignments, expr: Box::new(expression) };
            Ok(Expression::new(Expr::EnvShorthand(env_shorthand), span))
        }
    }
}

/// A keyword statement or, failing that, a command call (nu's
/// `parse_builtin_commands`). `if`, `loop`, ... are ordinary commands in nu,
/// so a definition in the file shadows them; the statement keywords cannot be
/// shadowed.
#[inline]
pub fn parse_builtin_commands<'a>(tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let Some(first) = tokens.peek_token() else {
        return Err(cut(Diagnostic::expected("command", tokens.end_span())));
    };
    if first.contents != TokenContents::Item {
        return Err(cut(Diagnostic::expected("command", first.span)));
    }
    let head = working_set.get_span_contents(first.span);
    let head = match is_statement_keyword(head) || !working_set.is_declared(head) {
        true => head,
        false => "",
    };
    // nu reads `if`, `match`, `while`, `loop`, `try`, `return`, `break` and `continue` with
    // `parse_call`, whose `find_longest_decl` takes a longer known name that starts with the
    // keyword (`def "if ready"`); `where` and the statement keywords go by their first word.
    let longer_name = || {
        let mut after_head = tokens;
        after_head.next_token();
        let (longest, _) = find_longest_decl(*first, &mut after_head, "");
        longest.span != first.span
    };
    let (context, result) = match head {
        "def" => ("def", parse_def(tokens)),
        "extern" => ("extern", parse_extern(tokens)),
        "let" => ("let", parse_let(tokens)),
        "mut" => ("mut", parse_mut(tokens)),
        "const" => ("const", parse_const(tokens)),
        "for" => ("for", parse_for(tokens)),
        "alias" => ("alias", parse_alias(tokens, false)),
        "module" => ("module", parse_module(tokens)),
        "use" => ("use", parse_use(tokens)),
        "export" => ("export", parse_export_in_block(tokens)),
        "export-env" => ("export-env", parse_export_env(tokens)),
        "if" | "match" | "while" | "loop" | "try" | "return" | "break" | "continue" if longer_name() => {
            ("command call", parse_call(tokens))
        }
        "if" => ("if", parse_if(tokens)),
        "match" => ("match", parse_match(tokens)),
        "while" => ("while", parse_while(tokens)),
        "loop" => ("loop", parse_loop(tokens)),
        "try" => ("try", parse_try(tokens)),
        "return" => ("return", parse_return(tokens)),
        "break" => ("break", parse_break_or_continue(tokens, Expr::Break)),
        "continue" => ("continue", parse_break_or_continue(tokens, Expr::Continue)),
        "where" => ("where", parse_where(tokens)),
        _ => ("command call", parse_call(tokens)),
    };
    result.map_err(|error| error.map(|failure| failure.with_context(context)))
}

/// The heads nu refuses in `parse_expression`, i.e. after a `|` or after
/// environment shorthand: declarations and the module/source commands
/// (`BuiltinCommandInPipeline`), `const`/`mut` (`AssignInPipeline`),
/// `overlay` unless the second item of the command is `list`, and `plugin`
/// when the second item is `use`. `tokens` is positioned at the head; its items
/// include any shorthand before it, as nu's `spans` do.
fn check_builtin_command_in_pipeline(tokens: &Tokens<'_, '_>) -> ParseResult<()> {
    let working_set = tokens.working_set;
    let Some(head) = tokens.peek_token() else { return Ok(()) };
    let text = working_set.get_span_contents(head.span);
    let second = tokens.all().get(1).map(|token| working_set.get_span_contents(token.span));
    let statement = match text {
        "def" | "extern" | "for" | "module" | "use" | "source" | "alias" | "export" | "export-env" | "hide" => true,
        "const" | "mut" => true,
        "overlay" => second != Some("list"),
        "plugin" => second == Some("use"),
        _ => false,
    };
    if statement {
        return Err(cut(Diagnostic::new(ErrorKind::KeywordInPipeline(text.to_string()), head.span).with_help(
            "declarations and the module, source, overlay and plugin commands are statements of their own",
        )));
    }
    Ok(())
}

/// Whether `name` can be set by `NAME=value` shorthand: an ASCII letter or `_`,
/// then letters, digits and `_` (nu's `is_env_variable_name`).
fn is_env_variable_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c == '_' || c.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

/// The `NAME=value` items before a command (nu's environment shorthand).
fn parse_env_shorthand_prefix<'a>(tokens: &mut Tokens<'_, 'a>) -> ParseResult<Vec<EnvAssignment<'a>>> {
    repeat(0.., env_assignment).parse_next(tokens)
}

/// One `NAME=value` item: an item with a valid variable name before its `=`
/// and a value that parses. Like nu, a value that does not parse (`A='b'c`)
/// ends the shorthand without an error: from that item on, the text is the
/// command (an external `A='b'c`).
fn env_assignment<'a>(tokens: &mut Tokens<'_, 'a>) -> ParseResult<EnvAssignment<'a>> {
    let working_set = tokens.working_set;
    let (token, equals) = item
        .verify_map(|token| {
            let text = working_set.get_span_contents(token.span);
            // This runs for every command, so only the name is scanned: up to the first byte
            // that cannot be part of one, which must be the `=`.
            let equals = text.bytes().position(|byte| byte != b'_' && !byte.is_ascii_alphanumeric())?;
            (text.as_bytes()[equals] == b'=' && is_env_variable_name(&text[..equals])).then_some(())?;
            Some((token, equals))
        })
        .parse_next(tokens)?;
    let text = tokens.text(&token);
    let value_span = Span::new(token.span.start + equals + 1, token.span.end);
    // Like nu, the value is a string unless it starts with `$`: `A=1` sets `A` to `"1"`.
    let value = match &text[equals + 1..] {
        "" => Ok(Expression::new(Expr::String(StringLiteral::bare("")), value_span)),
        value if value.starts_with('$') => parse_value(working_set, value_span, ExpectedShape::Any),
        _ => parse_string_literal(working_set, value_span)
            .map(|string| Expression::new(Expr::String(string), value_span)),
    };
    let value = value.map_err(|_| backtrack(token.span.start))?;
    let name = Spanned::new(&text[..equals], Span::new(token.span.start, token.span.start + equals));
    Ok(EnvAssignment { span: token.span, name, value })
}

/// `lhs = rhs` and the other assignment operators (nu's
/// `parse_assignment_expression`), where `rhs` is everything to the end of
/// the line.
fn parse_assignment_expression<'a>(tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let items = tokens.all();
    // The caller found an assignment operator among the items.
    let operator = items.iter().enumerate().find_map(|(index, token)| match token.contents {
        TokenContents::AssignmentOperator(operator) => Some((index, operator)),
        _ => None,
    });
    let Some((operator_index, operator)) = operator else {
        return Err(cut(Diagnostic::expected("assignment operator", tokens.here())));
    };
    let operator_token = items[operator_index];
    if operator_index == 0 {
        return Err(cut(Diagnostic::expected("left hand side of assignment", operator_token.span)));
    }
    let lhs = parse_expression(tokens.slice(0..operator_index), Position::Element)?;
    // nu accepts anything its `parse_full_cell_path` produces as the left side:
    // a variable, a subexpression, a list or a `key: value` record, with or
    // without a cell path (`(1) = 2`, `[1].0 = 2`), and fails at run time.
    let assignable = match &lhs.expr {
        Expr::Var(_) | Expr::Subexpression(_) | Expr::FullCellPath(_) => true,
        Expr::List(_) | Expr::Table(_) => true,
        Expr::Record(items) => matches!(items.first(), Some(RecordItem::Pair { .. })),
        _ => false,
    };
    if !assignable {
        return Err(cut(Diagnostic::message("assignment requires a variable", lhs.span)
            .with_help("only variables (`$x`) and their cell paths (`$x.a`, `$env.FOO`) can be assigned to")));
    }
    let rhs = tokens.slice(operator_index + 1..items.len());
    let Some(rhs_span) = rhs.span() else {
        return Err(cut(Diagnostic::expected("right hand side of assignment", operator_token.span.past())));
    };
    let rhs = parse_block(rhs, rhs_span);
    // Since 0.97 nu refuses an external command as the start of the value
    // unless it is written with a caret: `$x = git` is an error, `$x = ^git`
    // is not. A bare external head needs the command table (or an alias of
    // an external command), so without one this never fires.
    if let Some(first) = rhs.pipelines.first().and_then(|pipeline| pipeline.elements.first())
        && let Expr::ExternalCall(call) = &first.expr.expr
        && call.caret.is_none()
    {
        let name = working_set.get_span_contents(call.head.span);
        return Err(cut(Diagnostic::message("external command calls must be explicit in assignments", call.head.span)
            .with_help(format!(
                "`{name}` is not a known command; write `^{name}` to run it and capture its output, or quote the string"
            ))));
    }
    let span = lhs.span.merge(rhs_span);
    Ok(Expression::new(
        Expr::Assignment(Assignment { lhs: Box::new(lhs), op: Spanned::new(operator, operator_token.span), rhs }),
        span,
    ))
}

/// An operator item (nu's `parse_operator`), with hints for common mistakes.
fn parse_operator(working_set: &WorkingSet<'_>, token: &Token) -> ParseResult<Spanned<Operator>> {
    let text = working_set.get_span_contents(token.span);
    if let Some(operator) = Operator::from_spelling(text) {
        return Ok(Spanned::new(operator, token.span));
    }
    let help = match text {
        "^" | "pow" => "use `**` for exponentiation",
        "is" | "===" => "use `==` for equality",
        "contains" => "use `has` to test membership",
        "%" => "use `mod` for the remainder",
        "&" => "use `bit-and`",
        "<<" => "use `bit-shl`",
        ">>" => "use `bit-shr`",
        "bits-and" => "did you mean `bit-and`?",
        "bits-xor" => "did you mean `bit-xor`?",
        "bits-or" => "did you mean `bit-or`?",
        "bits-shl" => "did you mean `bit-shl`?",
        "bits-shr" => "did you mean `bit-shr`?",
        "!" => "use `not` for boolean negation",
        _ => return Err(cut(Diagnostic::expected("operator", token.span))),
    };
    Err(cut(Diagnostic::new(ErrorKind::UnknownOperator(text.to_string()), token.span).with_help(help)))
}

/// A math expression (nu's `parse_math_expression`): operands separated by
/// operators, with Nushell's precedence (all left-associative except `**`),
/// `not` prefixes, and `if` / `match` allowed as operands.
///
/// ```text
/// math-expression = operand { operator operand }
/// operand         = "if" ... | "match" ... | { "not" } value
/// ```
///
/// winnow's [`expression`] does the precedence climbing; [`infix_operator`]
/// gives each operator its binding power and associativity.
pub fn parse_math_expression<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let Some(first) = tokens.peek_token().filter(|token| token.contents == TokenContents::Item) else {
        return Err(cut(Diagnostic::expected("expression", tokens.here())));
    };
    if matches!(tokens.text(first), "if" | "match") {
        return parse_builtin_commands(tokens);
    }
    expression(parse_math_operand).infix(infix_operator).parse_next(&mut tokens)
}

/// The operator after an operand, with its binding power: `Left` for all but
/// the right-associative `**`. winnow asks for one only while tokens are left,
/// and the item after an operand must be an operator, so `1 2` is an error
/// rather than the end of the expression.
fn infix_operator<'t, 'a>(
    tokens: &mut Tokens<'t, 'a>,
) -> ParseResult<Infix<Tokens<'t, 'a>, Expression<'a>, ErrMode<ParseFailure>>> {
    let token = expected("operator", item).parse_next(tokens)?;
    let operator = parse_operator(tokens.working_set, &token)?.item;
    let power = i64::from(operator.precedence());
    Ok(match operator.is_right_associative() {
        true => Infix::Right(power, fold_binary_op),
        false => Infix::Left(power, fold_binary_op),
    })
}

/// Combine two operands with the operator between them. winnow hands the
/// fold only the operands, so the operator is found as the token right after
/// the left one.
fn fold_binary_op<'a>(
    tokens: &mut Tokens<'_, 'a>,
    lhs: Expression<'a>,
    rhs: Expression<'a>,
) -> ParseResult<Expression<'a>> {
    let token = tokens.token_after(lhs.span.end).expect("an operator follows its left operand");
    let operator = Operator::from_spelling(tokens.text(token)).expect("checked by `infix_operator`");
    let span = lhs.span.merge(rhs.span);
    let op = Spanned::new(operator, token.span);
    Ok(Expression::new(Expr::BinaryOp(BinaryOp { lhs: Box::new(lhs), op, rhs: Box::new(rhs) }), span))
}

/// A `where` condition (nu's `parse_row_condition`): a math expression in
/// which a bare string on the left of an operator, or alone, is a column of
/// the row, so `size > 1kb` means `$it.size > 1kb`.
pub fn parse_row_condition<'a>(tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let condition = parse_math_expression(tokens)?;
    expand_row_condition(working_set, condition)
}

/// Expand the left operand of every operator of a row condition, and a lone operand.
fn expand_row_condition<'a>(
    working_set: &WorkingSet<'a>,
    mut condition: Expression<'a>,
) -> ParseResult<Expression<'a>> {
    let Expr::BinaryOp(binary) = &mut condition.expr else {
        return expand_to_cell_path(working_set, condition);
    };
    replace_boxed(&mut binary.lhs, |lhs| expand_row_condition(working_set, lhs))?;
    // A right operand stays a value (`name == foo` compares with the string `foo`);
    // one that is an operation of its own has left operands to expand.
    if let Expr::BinaryOp(_) = binary.rhs.expr {
        replace_boxed(&mut binary.rhs, |rhs| expand_row_condition(working_set, rhs))?;
    }
    Ok(condition)
}

/// Replace the expression in `boxed` by `change` of it, reusing the box.
fn replace_boxed<'a>(
    boxed: &mut Box<Expression<'a>>,
    change: impl FnOnce(Expression<'a>) -> ParseResult<Expression<'a>>,
) -> ParseResult<()> {
    let placeholder = Expression::new(Expr::Garbage, boxed.span);
    let expression = std::mem::replace(&mut **boxed, placeholder);
    **boxed = change(expression)?;
    Ok(())
}

/// In a row condition, a string operand `size` means `$it.size`.
fn expand_to_cell_path<'a>(working_set: &WorkingSet<'a>, expr: Expression<'a>) -> ParseResult<Expression<'a>> {
    match expr.expr {
        // nu's `expand_to_cell_path` re-parses only an `Expr::String`: a raw string is nu's
        // `Expr::RawString`, and stays a value (`where r#'a'# == 'a'`).
        Expr::String(ref string) if !matches!(string.quote, Quote::Raw(_)) => {
            parse_full_cell_path(working_set, expr.span, true)
        }
        Expr::UnaryNot(mut not) => {
            replace_boxed(&mut not.expr, |operand| expand_to_cell_path(working_set, operand))?;
            Ok(Expression::new(Expr::UnaryNot(not), expr.span))
        }
        kind => Ok(Expression { span: expr.span, expr: kind }),
    }
}

/// One operand: `not* value`, or an `if`/`match` that takes the rest of the
/// items (`1 + if $x { 2 } else { 3 }`). As in nu, a `not` applies to the one
/// value after it: `not $a == $b` is `(not $a) == $b`.
fn parse_math_operand<'a>(tokens: &mut Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let Some(first) = tokens.peek_token() else {
        // Only after an operator: `parse_math_expression` checked the first operand.
        return Err(cut(Diagnostic::expected("expression after operator", Span::point(tokens.previous_token_end()))
            .with_help("this math expression is incomplete")));
    };
    if first.contents == TokenContents::Item && matches!(tokens.text(first), "if" | "match") {
        let keyword_expression = parse_builtin_commands(tokens.rest_stream())?;
        tokens.consume_rest();
        return Ok(keyword_expression);
    }
    let nots: Vec<Token> = repeat(0.., keyword("not")).parse_next(tokens)?;
    let value = tokens.expect_item("expression")?;
    let mut expression = parse_value(working_set, value.span, ExpectedShape::Any)?;
    for not in nots.into_iter().rev() {
        let span = not.span.merge(expression.span);
        expression = Expression::new(Expr::UnaryNot(UnaryNot { not_span: not.span, expr: Box::new(expression) }), span);
    }
    Ok(expression)
}

/// What the surrounding grammar expects an item to be: the `SyntaxShape` nu
/// hands `parse_value` for the argument position. It decides what `{ ... }`
/// is (closure, record or block), which literals a bare word may be, and what
/// `[` may start. Statement bodies (`if`, `def`, ...) never go through
/// [`parse_value`]: the statement parsers call [`parse_block_body`] directly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ExpectedShape<'t, 'a> {
    /// Anything.
    Any,
    /// A closure (`{|x| ...}` or `{ ... }`).
    Closure,
    /// A `match` arm body: a block, unless it is written as a closure or a
    /// record (nu's `OneOf(Block, Expression)`).
    MatchArmBody,
    /// A number (range bounds).
    Number,
    /// A string (record keys, module names, `record<...>` field names):
    /// `true`, `false` and `null` are refused and `[...]` is a bare word.
    String,
    /// The declared shape of a parameter whose default value this is
    /// (`[x: int = 1]`), or of the items of a `list<...>`: nu parses the value
    /// with that shape.
    Declared(&'t SyntaxShape<'a>),
}

/// One item, parsed as `shape` expects (nu's `parse_value`). How the item
/// starts picks its kind (`$`, `(`, `{`, `[`, `r#`), whose parser may still
/// consult the shape; a bare word is a literal of the shape.
pub fn parse_value<'a>(
    working_set: &WorkingSet<'a>,
    span: Span,
    shape: ExpectedShape<'_, 'a>,
) -> ParseResult<Expression<'a>> {
    let text = working_set.get_span_contents(span);
    if let ExpectedShape::Declared(declared) = shape {
        return parse_value_for_shape(working_set, span, declared);
    }
    match text.as_bytes() {
        [] => Err(cut(Diagnostic::expected("value", span))),
        [b'$', ..] => parse_dollar_expr(working_set, span),
        [b'(', ..] => parse_paren_expr(working_set, span),
        [b'{', ..] => parse_brace_expr(working_set, span, shape),
        // `parse_string` on `[a b]`: the text is a bare word.
        [b'[', ..] if shape == ExpectedShape::String => parse_string(working_set, span),
        [b'[', ..] if shape == ExpectedShape::Number => Err(cut(Diagnostic::expected("number", span))),
        [b'[', ..] => parse_full_cell_path(working_set, span, false),
        [b'r', b'#', ..] => parse_raw_string(working_set, span),
        _ => match shape {
            ExpectedShape::Number => parse_number(working_set, span),
            ExpectedShape::String => parse_string_value(working_set, span),
            ExpectedShape::MatchArmBody => Err(cut(Diagnostic::expected("block", span))),
            ExpectedShape::Closure => Err(cut(Diagnostic::expected("closure", span))),
            ExpectedShape::Any | ExpectedShape::Declared(_) => parse_any_value(working_set, span, text),
        },
    }
}

/// nu's `SyntaxShape::String`: the keywords are refused, everything else is a string.
fn parse_string_value<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Expression<'a>> {
    match working_set.get_span_contents(span) {
        word @ ("true" | "false" | "null") => Err(cut(Diagnostic::expected("string", span)
            .with_help(format!("`{word}` is a value; quote it to use it as a string")))),
        _ => parse_string(working_set, span),
    }
}

/// A bare word in value position: nu's order of literal kinds (`null`,
/// booleans, binary, range, filesize, duration, datetime, int, float), else a
/// string.
fn parse_any_value<'a>(working_set: &WorkingSet<'a>, span: Span, text: &'a str) -> ParseResult<Expression<'a>> {
    match text {
        "null" => return Ok(Expression::new(Expr::Nothing, span)),
        "true" => return Ok(Expression::new(Expr::Bool(true), span)),
        "false" => return Ok(Expression::new(Expr::Bool(false), span)),
        _ => {}
    }
    if let Some(binary) = parse_binary(working_set, span) {
        return binary;
    }
    if is_range_syntax(text) {
        return parse_range(working_set, span);
    }
    if let Some(filesize) = parse_filesize(text) {
        let filesize = filesize.map_err(|message| invalid_literal("filesize", message, span))?;
        return Ok(Expression::new(Expr::Filesize(filesize), span));
    }
    if let Some(duration) = parse_duration(text) {
        let duration = duration.map_err(|message| invalid_literal("duration", message, span))?;
        return Ok(Expression::new(Expr::Duration(duration), span));
    }
    if is_datetime(text) {
        return Ok(Expression::new(Expr::DateTime(text), span));
    }
    if let Some(int) = parse_int(text) {
        return Ok(Expression::new(Expr::Int(int), span));
    }
    // A radix prefix commits the word to being an int, as in Nushell: `0b2`
    // and `0x[13]=` are errors, not bare words.
    if let Some(radix) = radix_prefix(text) {
        return Err(invalid_literal("int", &format!("invalid digits for radix {radix}"), span));
    }
    if let Some(float) = parse_float(text) {
        return Ok(Expression::new(Expr::Float(float), span));
    }
    parse_string(working_set, span)
}

/// nu's `parse_value` with a declared shape: the value a parameter default
/// must be. `$`, `(` and `{` items are what they always are (a closure is
/// refused for a `block`), `[` is allowed for the list-like and string-like
/// shapes only, and a bare word must be a literal of the shape.
fn parse_value_for_shape<'a>(
    working_set: &WorkingSet<'a>,
    span: Span,
    declared: &SyntaxShape<'a>,
) -> ParseResult<Expression<'a>> {
    let text = working_set.get_span_contents(span);
    let name = shape_description(declared);
    let expected = || cut(Diagnostic::expected(name, span));
    match text.as_bytes() {
        [] => return Err(cut(Diagnostic::expected("value", span))),
        [b'$', ..] => return parse_dollar_expr(working_set, span),
        [b'(', ..] => return parse_paren_expr(working_set, span),
        [b'{', ..] => {
            // Only `closure` and `any` take a body of code; for the other shapes the `{`
            // must be a record or have closure parameters (`String` stands for "a value").
            let shape = match declared {
                SyntaxShape::Closure => ExpectedShape::Closure,
                SyntaxShape::Any => ExpectedShape::Any,
                _ => ExpectedShape::String,
            };
            return parse_brace_expr(working_set, span, shape).map_err(|error| {
                error.map(|failure| {
                    failure.map_diagnostic(|diagnostic| match diagnostic.kind {
                        ErrorKind::Expected(_) => {
                            Diagnostic::expected(name, span).with_help("found a block or closure")
                        }
                        _ => diagnostic,
                    })
                })
            });
        }
        [b'[', ..] => {
            return match declared {
                SyntaxShape::Any | SyntaxShape::Table(_) | SyntaxShape::ExternalArgument => {
                    parse_full_cell_path(working_set, span, false)
                }
                SyntaxShape::List(element) => parse_list_expression_with_shape(
                    working_set,
                    span,
                    element.as_deref().map(|element| &element.shape),
                ),
                SyntaxShape::String | SyntaxShape::Filepath | SyntaxShape::GlobPattern => {
                    parse_string(working_set, span)
                }
                SyntaxShape::OneOf(alternatives) => parse_oneof(working_set, span, alternatives),
                _ => Err(expected()),
            };
        }
        [b'r', b'#', ..] => return parse_raw_string(working_set, span),
        _ => {}
    }
    let literal = |expr: Expr<'a>| Ok(Expression::new(expr, span));
    match declared {
        SyntaxShape::Any => parse_any_value(working_set, span, text),
        SyntaxShape::Number => parse_number(working_set, span),
        SyntaxShape::Float => parse_float(text).map_or_else(|| Err(expected()), |float| literal(Expr::Float(float))),
        SyntaxShape::Int => match parse_int(text) {
            Some(int) => literal(Expr::Int(int)),
            // `0b2`: the error an untyped value gets, not a shape mismatch.
            None if radix_prefix(text).is_some() => parse_any_value(working_set, span, text),
            None => Err(expected()),
        },
        SyntaxShape::Duration => match parse_duration(text) {
            Some(Ok(duration)) => literal(Expr::Duration(duration)),
            Some(Err(message)) => Err(invalid_literal("duration", message, span)),
            None => Err(expected()),
        },
        SyntaxShape::Filesize => match parse_filesize(text) {
            Some(Ok(filesize)) => literal(Expr::Filesize(filesize)),
            Some(Err(message)) => Err(invalid_literal("filesize", message, span)),
            None => Err(expected()),
        },
        SyntaxShape::DateTime if is_datetime(text) => literal(Expr::DateTime(text)),
        SyntaxShape::Range if is_range_syntax(text) => parse_range(working_set, span),
        SyntaxShape::Boolean if text == "true" => literal(Expr::Bool(true)),
        SyntaxShape::Boolean if text == "false" => literal(Expr::Bool(false)),
        SyntaxShape::Nothing if text == "null" => literal(Expr::Nothing),
        SyntaxShape::String | SyntaxShape::Filepath | SyntaxShape::Directory | SyntaxShape::GlobPattern => {
            parse_string_value(working_set, span)
        }
        SyntaxShape::Binary => parse_binary(working_set, span).unwrap_or_else(|| Err(expected())),
        SyntaxShape::CellPath => parse_simple_cell_path(working_set, span),
        SyntaxShape::ExternalArgument => parse_external_string(working_set, span),
        SyntaxShape::OneOf(alternatives) => parse_oneof(working_set, span, alternatives),
        _ => Err(expected()),
    }
}

/// `oneof<a, b>` (nu's `parse_oneof`): the first shape the value parses as.
fn parse_oneof<'a>(
    working_set: &WorkingSet<'a>,
    span: Span,
    alternatives: &[TypeAnnotation<'a>],
) -> ParseResult<Expression<'a>> {
    let mut first_error = None;
    for alternative in alternatives {
        match parse_value_for_shape(working_set, span, &alternative.shape) {
            Ok(value) => return Ok(value),
            Err(error) => first_error.get_or_insert(error),
        };
    }
    Err(first_error.unwrap_or_else(|| cut(Diagnostic::expected("value", span))))
}

/// The word nu uses for a shape in "expected ..." errors.
fn shape_description(shape: &SyntaxShape<'_>) -> &'static str {
    match shape {
        SyntaxShape::Any => "any",
        SyntaxShape::Binary => "binary",
        SyntaxShape::Boolean => "bool",
        SyntaxShape::CellPath => "cell-path",
        SyntaxShape::Closure => "closure",
        SyntaxShape::DateTime => "datetime",
        SyntaxShape::Directory => "directory",
        SyntaxShape::Duration => "duration",
        SyntaxShape::Error => "error",
        SyntaxShape::ExternalArgument => "external argument",
        SyntaxShape::Float => "float",
        SyntaxShape::Filesize => "filesize with valid units",
        SyntaxShape::GlobPattern => "glob pattern",
        SyntaxShape::Int => "int",
        SyntaxShape::Nothing => "nothing",
        SyntaxShape::Number => "number",
        SyntaxShape::Filepath => "path",
        SyntaxShape::Range => "range",
        SyntaxShape::String => "string",
        SyntaxShape::List(_) => "list",
        SyntaxShape::Record(_) => "record",
        SyntaxShape::Table(_) => "table",
        SyntaxShape::OneOf(_) => "one of the accepted shapes",
    }
}

/// `true` if `text` is one of the things Nushell parses as the start of a math
/// expression rather than a command name (`is_math_expression_like`).
pub fn is_math_expression_like(text: &str) -> bool {
    match text.as_bytes() {
        [] => false,
        b"true" | b"false" | b"null" | b"not" | b"if" | b"match" => true,
        [b'r', b'#', ..] | [b'(' | b'{' | b'[' | b'$' | b'"' | b'\'' | b'-', ..] => true,
        _ => {
            parse_int(text).is_some()
                || parse_float(text).is_some()
                || parse_filesize(text).is_some_and(|filesize| filesize.is_ok())
                || parse_duration(text).is_some_and(|duration| duration.is_ok())
                || is_datetime(text)
                || looks_like_binary(text)
                || is_range_head(text)
        }
    }
}

/// Parse `( ... )` as a subexpression. Newlines inside are whitespace.
pub fn parse_subexpression<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Expression<'a>> {
    let inner = delimited_interior(working_set, span, "(", ")")?;
    let tokens = working_set.lex(inner, LexOptions::SUBEXPRESSION).map_err(cut)?;
    working_set.enter_scope();
    let block = parse_block(Tokens::from_lexed(working_set, &tokens), inner);
    working_set.exit_scope();
    Ok(Expression::new(Expr::Subexpression(block), span))
}

/// What the first two tokens of a `{ ... }` body say about it (nu's
/// `parse_brace_expr` looks at the same two tokens).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BraceShape {
    /// `{}`, only whitespace and comments, or a body whose first tokens do not lex.
    Empty,
    /// Starts with `|` or `||`: closure parameters.
    ClosureParams,
    /// `key:`: a record.
    Record,
    /// `...spread`: a record in value position, a block or closure elsewhere.
    Spread,
    /// Anything else: code.
    Other,
}

impl BraceShape {
    /// The shape that `probe`, the first two tokens of a `{` body lexed from `source` with
    /// [`LexOptions::BRACE_PROBE`], gives the body. The arms are in nu's order: pipes, then
    /// `key:`, then a spread.
    pub fn of_probe(probe: &[Token], source: &str) -> BraceShape {
        match probe {
            [first, ..] if matches!(first.contents, TokenContents::Pipe | TokenContents::PipePipe) => {
                BraceShape::ClosureParams
            }
            [_, second, ..] if second.text(source) == ":" => BraceShape::Record,
            [first, ..] if first.contents == TokenContents::Item && is_spread(first.text(source), b"{$(") => {
                BraceShape::Spread
            }
            [first, ..] if first.contents != TokenContents::Eof => BraceShape::Other,
            _ => BraceShape::Empty,
        }
    }
}

/// The [`BraceShape`] of `inner`, the text inside a `{`, from its first two
/// tokens ([`BraceShape::of_probe`]). [`LexOptions::BRACE_PROBE`] makes newlines
/// whitespace, skips comments and splits off `:`, so these are the first two
/// tokens of substance and `a:` reads as a key.
///
/// The probe only picks a parser and, like nu, reports no lex error: one reads
/// as `Empty` (`unwrap_or_default`), and the parser chosen for an `Empty` body
/// lexes it again, where a real error is reported.
fn probe_brace_shape(working_set: &WorkingSet<'_>, inner: Span) -> BraceShape {
    let probe = working_set.lex_n_tokens(inner, LexOptions::BRACE_PROBE, 2).unwrap_or_default();
    BraceShape::of_probe(&probe, working_set.source)
}

/// The shape of a `{ ... }` item (checking that it closes).
pub fn brace_shape(working_set: &WorkingSet<'_>, span: Span) -> ParseResult<BraceShape> {
    let inner = delimited_interior(working_set, span, "{", "}")?;
    Ok(probe_brace_shape(working_set, inner))
}

/// The text between the first and the last character of a `{` item, which is
/// what nu's `parse_brace_expr` probes: a tail such as the `.a` of `{a: 1}.a`
/// is included. nu cuts one byte; here the whole last character, so that a
/// multi-byte one (`{a: 1}╮`) is not split.
fn brace_probe_interior(working_set: &WorkingSet<'_>, span: Span) -> Span {
    let last = working_set.get_span_contents(span).chars().next_back().map_or(0, char::len_utf8);
    Span::new(span.start + 1, (span.end - last).max(span.start + 1))
}

/// A `{` item that nu reads as a record, or as a cell path on one, whatever
/// shape it wants (nu's `parse_brace_expr` decides this first): the second
/// token of [`brace_probe_interior`] is `:` (`{a: 1}`, `{ $env.A:b }`,
/// `{a: 1}.a`). `None` for anything else.
pub fn parse_brace_record<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Option<Expression<'a>>> {
    if probe_brace_shape(working_set, brace_probe_interior(working_set, span)) != BraceShape::Record {
        return Ok(None);
    }
    parse_full_cell_path(working_set, span, false).map(Some)
}

/// Whether nu types this record `record<...>`: every key a string that is not
/// raw (nu's `Expr::String`) and every spread such a record. Any other record
/// (`{ $env.A: b }`, `{(k): 1}`), and a cell path on one, is `any` to nu.
pub fn is_typed_record(expr: &Expression<'_>) -> bool {
    let Expr::Record(items) = &expr.expr else { return false };
    items.iter().all(|item| match item {
        RecordItem::Pair { key, .. } => {
            matches!(&key.expr, Expr::String(string) if !matches!(string.quote, Quote::Raw(_)))
        }
        RecordItem::Spread { expr, .. } => is_typed_record(expr),
    })
}

/// `{ ... }`: a record, a closure or a block, decided as `nu-parser` does
/// from the first two tokens of the body and the surrounding shape.
fn parse_brace_expr<'a>(
    working_set: &WorkingSet<'a>,
    span: Span,
    shape: ExpectedShape<'_, 'a>,
) -> ParseResult<Expression<'a>> {
    let text = working_set.get_span_contents(span);
    if !text.ends_with('}') {
        // `{a: 1}.a`. Like nu, the kind is decided from the text between the
        // first and the last character, tail included: a record (`key :`) or a
        // first token `}` (`{}.a`) is a cell path, anything else a closure,
        // block or record that never closes (`{#a: 1}.a`, `{...$r}.a`). The
        // lexer refuses a `}` that was never opened, so the probe cannot see a
        // first `}`: the lines are scanned for it, past blanks and comments.
        let inner = brace_probe_interior(working_set, span);
        let rest = working_set.get_span_contents(inner);
        let first_is_close = rest
            .lines()
            .map(str::trim_start)
            .find(|line| !line.is_empty() && !line.starts_with('#'))
            .is_some_and(|line| line.starts_with('}'));
        if first_is_close || probe_brace_shape(working_set, inner) == BraceShape::Record {
            return parse_full_cell_path(working_set, span, false);
        }
        let open = Span::new(span.start, span.start + 1);
        return Err(cut(Diagnostic::new(ErrorKind::Unclosed { delimiter: "}", open }, span.past())));
    }
    let inner = Span::new(span.start + 1, span.end - 1);
    match (probe_brace_shape(working_set, inner), shape) {
        (BraceShape::Empty, ExpectedShape::Closure) => parse_closure_expression(working_set, span),
        (BraceShape::Empty, ExpectedShape::MatchArmBody) => parse_block_expression(working_set, span),
        (BraceShape::Empty, _) => parse_record(working_set, span),
        (BraceShape::ClosureParams, _) => parse_closure_expression(working_set, span),
        (BraceShape::Record, _) => parse_record(working_set, span),
        (BraceShape::Spread, ExpectedShape::Closure) => parse_closure_expression(working_set, span),
        (BraceShape::Spread, ExpectedShape::MatchArmBody) => parse_block_expression(working_set, span),
        (BraceShape::Spread, _) => parse_record(working_set, span),
        (BraceShape::Other, ExpectedShape::MatchArmBody) => parse_block_expression(working_set, span),
        (BraceShape::Other, ExpectedShape::Closure | ExpectedShape::Any) => parse_closure_expression(working_set, span),
        (BraceShape::Other, ExpectedShape::Number | ExpectedShape::String | ExpectedShape::Declared(_)) => {
            Err(cut(Diagnostic::expected("value", span).with_help("found a block or closure")))
        }
    }
}

/// Parse `{ ... }` as a block: no parameters, and not a record.
pub fn parse_block_body<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Block<'a>> {
    let inner = delimited_interior(working_set, span, "{", "}")?;
    match probe_brace_shape(working_set, inner) {
        BraceShape::ClosureParams => {
            return Err(cut(Diagnostic::expected("block", span)
                .with_help("found closure parameters; blocks cannot have parameters")));
        }
        BraceShape::Record => {
            return Err(cut(Diagnostic::expected("block", span).with_help("found a record")));
        }
        BraceShape::Empty | BraceShape::Spread | BraceShape::Other => {}
    }
    parse_block_body_unchecked(working_set, span)
}

/// Parse `{ ... }` as a block without looking at its shape (the body of a
/// `def`, which nu parses as a closure whatever it starts with).
pub fn parse_block_body_unchecked<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Block<'a>> {
    let inner = delimited_interior(working_set, span, "{", "}")?;
    let tokens = working_set.lex(inner, LexOptions::BLOCK).map_err(cut)?;
    working_set.enter_scope();
    let block = parse_block(Tokens::from_lexed(working_set, &tokens), inner);
    working_set.exit_scope();
    Ok(block)
}

/// [`parse_block_body`] as an expression: a `{ ... }` match-arm body (nu's `parse_block_expression`).
fn parse_block_expression<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Expression<'a>> {
    Ok(Expression::new(Expr::Block(parse_block_body(working_set, span)?), span))
}

/// Parse `{|params| body}` or `{ body }` as a closure (nu's `parse_closure_expression`).
pub fn parse_closure_expression<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Expression<'a>> {
    Ok(Expression::new(Expr::Closure(Box::new(parse_closure_parts(working_set, span)?)), span))
}

/// The parameters and body of a closure item; `params` is `None` without pipes.
///
/// ```text
/// closure = "{" [ "|" parameters "|" | "||" ] block "}"
/// ```
pub fn parse_closure_parts<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Closure<'a>> {
    let inner = delimited_interior(working_set, span, "{", "}")?;
    let tokens = working_set.lex(inner, LexOptions::BLOCK).map_err(cut)?;
    // The parameter list is the first token or, as nu's lexer merges a newline
    // and the `|` after it, the `|` right after one newline (`{\n  |x| ... }`).
    // After a comment line or a blank line, and for `||` on a later line, the
    // pipes belong to the body (`{\n|| 1}` is `||` in a block).
    let first = match (tokens.first().map(|token| token.contents), tokens.get(1).map(|token| token.contents)) {
        (Some(TokenContents::Eol), Some(TokenContents::Pipe)) => 1,
        _ => 0,
    };
    let (params, body_start) = match tokens.get(first).map(|token| token.contents) {
        Some(TokenContents::Pipe) => {
            let open = tokens[first];
            let close_index = tokens
                .iter()
                .skip(first + 1)
                .position(|token| token.contents == TokenContents::Pipe)
                .map(|index| index + first + 1);
            let Some(close_index) = close_index else {
                return Err(cut(Diagnostic::new(
                    ErrorKind::Unclosed { delimiter: "|", open: open.span },
                    inner.past(),
                )
                .with_context("closure parameters")));
            };
            let close = tokens[close_index];
            // Like nu, the parameters are lexed with their pipes, so a trailing
            // `=` or `:` is not the last token (`{|x=| }` parses).
            let pipes = open.span.merge(close.span);
            let signature = parse_signature_helper(working_set, pipes, pipes, false)?;
            (Some(signature), close_index + 1)
        }
        Some(TokenContents::PipePipe) => {
            (Some(Signature { span: tokens[first].span, ..Signature::default() }), first + 1)
        }
        _ => (None, 0),
    };
    let body_tokens = &tokens[body_start..];
    let body_span = Span::new(body_tokens.first().map_or(inner.end, |token| token.span.start), inner.end);
    working_set.enter_scope();
    let body = parse_block(Tokens::from_lexed(working_set, body_tokens), body_span);
    working_set.exit_scope();
    Ok(Closure { params, body })
}

/// The tokens of a `[...]` interior, comments recorded and dropped.
fn lex_bracket_interior(working_set: &WorkingSet<'_>, span: Span) -> ParseResult<Vec<Token>> {
    let inner = delimited_interior(working_set, span, "[", "]")?;
    let tokens = working_set.lex(inner, LexOptions::LIST).map_err(cut)?;
    working_set.add_comments(&tokens);
    Ok(tokens
        .into_iter()
        .filter(|token| !matches!(token.contents, TokenContents::Comment | TokenContents::Eol | TokenContents::Eof))
        .collect())
}

/// Like nu, a `;` between list items is an error (only `[[cols]; [row]]` has one).
fn reject_semicolon(items: &[Token], what: &'static str) -> ParseResult<()> {
    match items.iter().find(|token| token.contents == TokenContents::Semicolon) {
        Some(token) => Err(cut(Diagnostic::message(format!("unexpected semicolon in {what}"), token.span)
            .with_help("use commas or whitespace to separate list items"))),
        None => Ok(()),
    }
}

/// `[ ... ]` as a list or, when it is `[[cols]; [row] ...]`, a table (nu's
/// `parse_list_expression`). Commas and newlines are whitespace.
///
/// ```text
/// list  = "[" { value | "..." value } "]"
/// table = "[" "[" { column } "]" ";" row { row } "]"
/// row   = "[" { value } "]"                            one value per column
/// ```
pub fn parse_list_expression<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Expression<'a>> {
    parse_list_expression_with_shape(working_set, span, None)
}

/// [`parse_list_expression`] with the items parsed as `element` (the default
/// of a `list<int>` parameter).
fn parse_list_expression_with_shape<'a>(
    working_set: &WorkingSet<'a>,
    span: Span,
    element: Option<&SyntaxShape<'a>>,
) -> ParseResult<Expression<'a>> {
    let items = lex_bracket_interior(working_set, span)?;
    if let [first, second, rows @ ..] = items.as_slice()
        && first.contents == TokenContents::Item
        && working_set.get_span_contents(first.span).starts_with('[')
        && second.contents == TokenContents::Semicolon
    {
        return parse_table_expression(working_set, span, first, second, rows);
    }
    reject_semicolon(&items, "list")?;
    let mut list_items = Vec::with_capacity(items.len());
    // Like nu, the interior is lite-parsed as a pipeline and the items of all its
    // commands are the list's: `[a | b]` is `[a b]`.
    for group in lite_parse_parts(working_set, &items)? {
        for token in &group {
            list_items.push(parse_list_item(working_set, token, element)?);
        }
    }
    Ok(Expression::new(Expr::List(list_items), span))
}

/// `[[cols]; [row] ...]` (nu's `parse_table_expression`): every row must be a
/// list with as many items as there are columns, and every column name must
/// be a string.
fn parse_table_expression<'a>(
    working_set: &WorkingSet<'a>,
    span: Span,
    columns: &Token,
    semicolon: &Token,
    rows: &[Token],
) -> ParseResult<Expression<'a>> {
    // The header and each row are lists in the tree.
    let list = |cells: Vec<Expression<'a>>, span: Span| {
        Expression::new(Expr::List(cells.into_iter().map(ListItem::Item).collect()), span)
    };
    let column_cells = parse_table_row(working_set, columns.span)?;
    if rows.is_empty() {
        return Err(cut(Diagnostic::expected("table row", semicolon.span.past())));
    }
    let width = column_cells.len();
    let rows = rows
        .iter()
        .map(|token| {
            if token.contents != TokenContents::Item || !working_set.get_span_contents(token.span).starts_with('[') {
                return Err(cut(
                    Diagnostic::message("table item not list", token.span).with_help("all table items must be lists")
                ));
            }
            let cells = parse_table_row(working_set, token.span)?;
            match cells.len().cmp(&width) {
                std::cmp::Ordering::Less => Err(cut(Diagnostic::message("missing columns", token.span)
                    .with_help(format!("expected {width} columns, found {}", cells.len())))),
                std::cmp::Ordering::Greater => {
                    let extra = cells[width].span.merge(cells[cells.len() - 1].span);
                    Err(cut(Diagnostic::message("extra columns", extra)
                        .with_help(format!("expected {width} columns, found {}", cells.len()))))
                }
                std::cmp::Ordering::Equal => Ok(list(cells, token.span)),
            }
        })
        .collect::<ParseResult<Vec<_>>>()?;
    for column in &column_cells {
        let stringy = matches!(
            column.expr,
            Expr::String(_)
                | Expr::StringInterpolation(_)
                | Expr::Var(_)
                | Expr::FullCellPath(_)
                | Expr::CellPath(_)
                | Expr::Subexpression(_)
        );
        if !stringy {
            return Err(cut(Diagnostic::message("table column name not string", column.span)
                .with_help("table column names should be able to be converted into strings")));
        }
    }
    let columns = list(column_cells, columns.span);
    Ok(Expression::new(Expr::Table(Table { columns: Box::new(columns), rows }), span))
}

/// One item of a list or table row, as [`lite_parse_parts`] leaves it: a
/// `...` spread, a value of the `element` shape (any shape when `None`), or a
/// token that is not an item (`=`, and a `|` or `o>` after one) as a bare word.
fn parse_list_item<'a>(
    working_set: &WorkingSet<'a>,
    token: &Token,
    element: Option<&SyntaxShape<'a>>,
) -> ParseResult<ListItem<'a>> {
    let text = working_set.get_span_contents(token.span);
    if token.contents == TokenContents::Item && is_spread(text, b"[$(") {
        let dots = Span::new(token.span.start, token.span.start + 3);
        let expr = parse_value(working_set, Span::new(token.span.start + 3, token.span.end), ExpectedShape::Any)?;
        return Ok(ListItem::Spread { dots, expr });
    }
    // `[Assignment, =, Assign]`: an operator on its own is just a word here,
    // and so is anything after it (`[a = b | c]`).
    if token.contents != TokenContents::Item {
        return Ok(ListItem::Item(Expression::new(Expr::String(crate::ast::StringLiteral::bare(text)), token.span)));
    }
    let shape = match element {
        Some(element) => ExpectedShape::Declared(element),
        None => ExpectedShape::Any,
    };
    Ok(ListItem::Item(parse_value(working_set, token.span, shape)?))
}

/// The cells of a table header or row (nu's `parse_table_row`): a list without spreads.
fn parse_table_row<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Vec<Expression<'a>>> {
    let items = lex_bracket_interior(working_set, span)?;
    reject_semicolon(&items, "list")?;
    let mut cells = Vec::new();
    for group in lite_parse_parts(working_set, &items)? {
        for token in &group {
            match parse_list_item(working_set, token, None)? {
                ListItem::Item(cell) => cells.push(cell),
                ListItem::Spread { dots, .. } => {
                    return Err(cut(Diagnostic::message("cannot spread in a table row", dots)));
                }
            }
        }
    }
    Ok(cells)
}

/// `{ key: value, ...$spread }` (nu's `parse_record`).
///
/// ```text
/// record = "{" { "..." value | key ":" value } "}"
/// ```
///
/// The interior is lexed one token at a time from a character stream: a key
/// with `:` as a token of its own (so `a:1` splits), a value without (so
/// `http://x` stays whole). Like nu, a key or a value must be an item:
/// `{a: =}` and `{a: o>}` are errors.
pub fn parse_record<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Expression<'a>> {
    let inner = delimited_interior(working_set, span, "{", "}")?;
    let mut interior = input(working_set.get_span_contents(inner), inner.start);
    let mut items = Vec::new();
    while let Some(first) = next_record_token(working_set, &mut interior, LexOptions::RECORD_KEY)? {
        items.push(parse_record_item(working_set, &mut interior, first)?);
    }
    Ok(Expression::new(Expr::Record(items), span))
}

/// The next token of a record's interior, lexed with `options`; comments are
/// recorded and passed over.
fn next_record_token(
    working_set: &WorkingSet<'_>,
    interior: &mut Input<'_>,
    options: LexOptions,
) -> ParseResult<Option<Token>> {
    while let Some(token) = working_set.next_token(interior, options)? {
        match token.contents {
            TokenContents::Comment => working_set.add_comment(token.span),
            _ => return Ok(Some(token)),
        }
    }
    Ok(None)
}

/// One entry of a record, starting at `first`: `...spread` or `key: value`.
fn parse_record_item<'a>(
    working_set: &WorkingSet<'a>,
    interior: &mut Input<'_>,
    first: Token,
) -> ParseResult<RecordItem<'a>> {
    if first.contents != TokenContents::Item {
        return Err(cut(Diagnostic::message("unexpected token in record", first.span)
            .with_help("expected a record key here; fields look like `key: value`")));
    }
    if is_spread(working_set.get_span_contents(first.span), b"{$(") {
        let dots = Span::new(first.span.start, first.span.start + 3);
        let expr = parse_value(working_set, Span::new(first.span.start + 3, first.span.end), ExpectedShape::Any)?;
        return Ok(RecordItem::Spread { dots, expr });
    }
    let key = parse_value(working_set, first.span, ExpectedShape::String)?;
    check_record_key_or_value(working_set, &key, "key")?;
    let colon = match next_record_token(working_set, interior, LexOptions::RECORD_KEY)? {
        Some(colon) if working_set.get_span_contents(colon.span) == ":" => colon,
        Some(other) => {
            return Err(cut(Diagnostic::expected("`:` after record key", other.span).with_help(
                "record fields look like `key: value`; a missing colon often makes this parse as a block or closure",
            )));
        }
        None => {
            return Err(cut(Diagnostic::expected("`:` after record key", first.span.past())
                .with_help("record fields look like `key: value`")));
        }
    };
    let value = match next_record_token(working_set, interior, LexOptions::RECORD_VALUE)? {
        Some(token) if token.contents == TokenContents::Item => {
            parse_value(working_set, token.span, ExpectedShape::Any)?
        }
        Some(token) => {
            return Err(cut(Diagnostic::message("unexpected token in record value", token.span)
                .with_help("after `key:`, provide a value (string, number, record, list, ...)")));
        }
        None => return Err(cut(Diagnostic::expected("value for record field", colon.span.past()))),
    };
    check_record_key_or_value(working_set, &value, "value")?;
    Ok(RecordItem::Pair { key, colon: colon.span, value })
}

/// Like Nushell, refuse a bare word containing `:` as a record key or value
/// (`{a: x:y}`, `{a: http://x}`): it is almost always a missing quote or
/// separator, and the lexer would otherwise split it unpredictably.
fn check_record_key_or_value(
    working_set: &WorkingSet<'_>,
    expr: &Expression<'_>,
    position: &'static str,
) -> ParseResult<()> {
    let bare_spans: Vec<Span> = match &expr.expr {
        Expr::String(string) if string.quote == Quote::Bare => vec![expr.span],
        Expr::StringInterpolation(interpolation) if interpolation.quote == Quote::Bare => interpolation
            .parts
            .iter()
            .filter_map(|part| match part {
                InterpolationPart::Text { span, .. } => Some(*span),
                InterpolationPart::Expression(_) => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    for span in bare_spans {
        if let Some(at) = working_set.get_span_contents(span).find(':') {
            let colon = Span::new(span.start + at, span.start + at + 1);
            return Err(cut(Diagnostic::message(format!("colon in bare word specifying record {position}"), colon)
                .with_help(format!("quote the {position} if the `:` is part of it"))));
        }
    }
    Ok(())
}

/// Parse the `{ pattern => body, ... }` item of a `match` (nu's
/// `parse_match_block_expression`): arms up to the closing brace.
///
/// ```text
/// match-block = "{" { arm } "}"
/// arm         = pattern { "|" pattern } [ "if" guard... ] "=>" body
/// ```
///
/// Commas and newlines are whitespace. A body is one token, so the next arm
/// starts right after it.
pub fn parse_match_block_expression<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Vec<MatchArm<'a>>> {
    let inner = delimited_interior(working_set, span, "{", "}")?;
    let lexed = working_set.lex(inner, LexOptions::MATCH).map_err(cut)?;
    working_set.add_comments(&lexed);
    // nu reads the arms by the text of each token, so a word that lexes as an
    // operator or a redirection (`=`, `o>`) is an ordinary pattern or body.
    let lexed: Vec<Token> = lexed
        .into_iter()
        .filter(|token| !matches!(token.contents, TokenContents::Comment | TokenContents::Eol | TokenContents::Eof))
        .map(|token| match token.contents {
            TokenContents::AssignmentOperator(_) | TokenContents::Redirection(_) => {
                Token { contents: TokenContents::Item, span: token.span }
            }
            _ => token,
        })
        .collect();
    let mut tokens = Tokens::new(working_set, &lexed, inner.end);
    repeat_till(0.., parse_match_arm, eof).map(|(arms, _)| arms).parse_next(&mut tokens)
}

/// `pattern ( | pattern )* [if guard...] => body`.
fn parse_match_arm<'a>(tokens: &mut Tokens<'_, 'a>) -> ParseResult<MatchArm<'a>> {
    let pattern = parse_or_pattern(tokens)?;
    let guard = opt(parse_match_guard).parse_next(tokens)?;
    let arrow = expected("`=>`", keyword("=>")).parse_next(tokens)?;
    let body = parse_match_arm_body(tokens)?;
    Ok(MatchArm { span: pattern.span.merge(body.span), pattern, guard, arrow: arrow.span, body })
}

/// A pattern, or alternatives separated by `|`.
fn parse_or_pattern<'a>(tokens: &mut Tokens<'_, 'a>) -> ParseResult<MatchPattern<'a>> {
    let first = expected("pattern", pattern).parse_next(tokens)?;
    let mut alternatives: Vec<MatchPattern<'a>> =
        repeat(0.., preceded(pipe, expected("pattern after `|`", pattern))).parse_next(tokens)?;
    let Some(last) = alternatives.last() else { return Ok(first) };
    let span = first.span.merge(last.span);
    alternatives.insert(0, first);
    Ok(MatchPattern { span, pattern: Pattern::Or(alternatives) })
}

/// `if condition...`: a guard, running up to the `=>`.
fn parse_match_guard<'a>(tokens: &mut Tokens<'_, 'a>) -> ParseResult<Box<Expression<'a>>> {
    let if_keyword = keyword("if").parse_next(tokens)?;
    let condition = tokens_until("=>").parse_next(tokens)?;
    if condition.at_end() {
        return Err(cut(Diagnostic::expected("expression after `if` in match guard", if_keyword.span.past())
            .with_help("the `if` keyword must be followed by an expression")));
    }
    // Like nu, every token up to the `=>` is a word of the guard (`$a if ; => 1`).
    let items: Vec<Token> =
        condition.remaining().iter().map(|token| Token { contents: TokenContents::Item, span: token.span }).collect();
    let end = condition.end_span().start;
    Ok(Box::new(parse_math_expression(Tokens::new(tokens.working_set, &items, end))?))
}

/// The body of an arm is one item: a block (or a record or closure when it
/// looks like one), otherwise an expression.
fn parse_match_arm_body<'a>(tokens: &mut Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    // Like nu, the body is the next token whatever it is: `1 => |` and `1 => ;`
    // are calls to commands named `|` and `;`.
    let Some(&body) = tokens.next_token() else {
        return Err(cut(Diagnostic::expected("match arm body", tokens.here())));
    };
    // An assignment operator is still one (`1 => =` is an assignment with no sides).
    if assignment_operator(tokens.working_set.get_span_contents(body.span)).is_some() {
        return Err(cut(Diagnostic::expected("match arm body", body.span)));
    }
    let body = Token { contents: TokenContents::Item, span: body.span };
    match tokens.text(&body).starts_with('{') {
        true => parse_value(tokens.working_set, body.span, ExpectedShape::MatchArmBody),
        false => parse_expression(
            Tokens::new(tokens.working_set, std::slice::from_ref(&body), body.span.end),
            Position::Element,
        ),
    }
}
