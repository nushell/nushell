#![allow(clippy::byte_char_slices)]

use crate::{
    TokenContents,
    lex_once::lex_span,
    lite_parse,
    parse_helpers::is_variable,
    parser::{ensure_not_reserved_variable_name, parse_value, unescape_unquote_string},
    type_check::type_compatible,
};
use nu_protocol::{
    ParseError, Span, SyntaxShape, Type, Value, VarId,
    ast::{Expr, MatchPattern, Pattern},
    engine::StateWorkingSet,
    eval_const::eval_constant,
};
pub fn garbage(span: Span) -> MatchPattern {
    MatchPattern {
        pattern: Pattern::Garbage,
        guard: None,
        span,
    }
}

/// Where a pattern is written, which decides how it declares its variables.
#[derive(Clone, Copy)]
enum PatternContext {
    /// A `match` arm, which has its own scope. A variable already declared there is reused, so
    /// both sides of an or-pattern bind the same variable.
    Match,
    /// A destructuring `let`, `mut`, `const` or `for`. Every variable is a new one, mutable for
    /// `mut`.
    Binding { mutable: bool },
}

/// Parse the pattern of a `match` arm.
pub fn parse_pattern(working_set: &mut StateWorkingSet, span: Span) -> MatchPattern {
    parse_pattern_in(working_set, span, PatternContext::Match)
}

/// Parse the pattern of a destructuring `let`, `mut`, `const` or `for`, as in
/// `let {$name, $size} = $file`.
///
/// It may hold `$name`, `_`, `?`, and list and record patterns with their rests, but no literals:
/// a literal would test the value rather than bind it, which is what `match` is for. Each variable
/// may appear once.
pub fn parse_binding_pattern(
    working_set: &mut StateWorkingSet,
    span: Span,
    mutable: bool,
) -> MatchPattern {
    let pattern = parse_pattern_in(working_set, span, PatternContext::Binding { mutable });
    check_binding_pattern(working_set, &pattern, &mut vec![]);
    pattern
}

/// Report the literals and repeated variables in a binding pattern. `bound` holds the names of
/// the variables seen so far.
fn check_binding_pattern(
    working_set: &mut StateWorkingSet,
    pattern: &MatchPattern,
    bound: &mut Vec<Vec<u8>>,
) {
    let name_span = match &pattern.pattern {
        Pattern::Variable(_) => pattern.span,
        // `..$name`
        Pattern::Rest(_) => Span::new(pattern.span.start + 2, pattern.span.end),
        Pattern::List(items) => {
            for item in items {
                check_binding_pattern(working_set, item, bound);
            }
            return;
        }
        Pattern::Record(fields) => {
            for (_, field) in fields {
                check_binding_pattern(working_set, field, bound);
            }
            return;
        }
        Pattern::Value(value) => {
            let text = working_set.get_span_contents(pattern.span);
            let bare_word = matches!(value, Value::String { .. })
                && text
                    .first()
                    .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_');
            let error = if bare_word {
                let text = String::from_utf8_lossy(text);
                ParseError::LabeledErrorWithHelp {
                    error: "A bare word in a pattern is a string, not a variable".into(),
                    label: format!("the string \"{text}\""),
                    help: format!("Write `${text}` to bind a variable."),
                    span: pattern.span,
                }
            } else {
                ParseError::LabeledErrorWithHelp {
                    error: "A destructuring pattern can't hold a literal".into(),
                    label: "a literal value".into(),
                    help: "Use `match` to compare a value with a literal.".into(),
                    span: pattern.span,
                }
            };
            working_set.error(error);
            return;
        }
        Pattern::Optional { pattern, .. } => {
            return check_binding_pattern(working_set, pattern, bound);
        }
        // Or-patterns only come from `match` arms.
        Pattern::Or(_) | Pattern::IgnoreValue | Pattern::IgnoreRest | Pattern::Garbage => return,
    };
    let name = working_set.get_span_contents(name_span).to_vec();
    if bound.contains(&name) {
        working_set.error(ParseError::LabeledErrorWithHelp {
            error: format!(
                "`{}` is bound twice in this pattern",
                String::from_utf8_lossy(&name)
            ),
            label: "bound again here".into(),
            help: "Give each variable its own name, or write `_` to skip a value.".into(),
            span: pattern.span,
        });
    } else {
        bound.push(name);
    }
}

/// Check a destructuring pattern against the type of the value it binds, and give its variables
/// their types.
///
/// A value that can't be a record, for a record pattern, or a list, for a list pattern, is an
/// error at `value_span`, as in `let {$a} = 5`. A field the type doesn't name may still be there
/// at run time, so that is left to the matcher. Fields and items take their types from `ty` only
/// with the `cell-path-types` experimental option, as `$value.field` does; they are `any`
/// otherwise.
pub fn check_binding_type(
    working_set: &mut StateWorkingSet,
    pattern: &MatchPattern,
    ty: &Type,
    value_span: Span,
) {
    let expected = match &pattern.pattern {
        Pattern::Record(_) => Type::record(),
        Pattern::List(_) => Type::list(Type::Any),
        _ => return,
    };
    if !type_compatible(&expected, ty) {
        working_set.error(ParseError::TypeMismatch(expected, ty.clone(), value_span));
    } else if nu_experimental::CELL_PATH_TYPES.get() {
        set_pattern_types(working_set, pattern, ty);
    }
}

/// Give each variable in `pattern` the type of the part of a `ty` value it binds.
fn set_pattern_types(working_set: &mut StateWorkingSet, pattern: &MatchPattern, ty: &Type) {
    match &pattern.pattern {
        Pattern::Variable(var_id) => working_set.set_variable_type(*var_id, ty.clone()),
        Pattern::Record(fields) => {
            for (name, field) in fields {
                let field_ty = match ty {
                    Type::Record(columns) => columns.get(name).cloned().unwrap_or(Type::Any),
                    _ => Type::Any,
                };
                set_pattern_types(working_set, field, &field_ty);
            }
        }
        Pattern::List(items) => {
            let (item_ty, rest_ty) = match ty {
                Type::List(item) => (item.as_ref().clone(), ty.clone()),
                Type::Table(columns) => (Type::Record(columns.clone()), ty.clone()),
                _ => (Type::Any, Type::list(Type::Any)),
            };
            for item in items {
                match item.pattern {
                    Pattern::Rest(var_id) => working_set.set_variable_type(var_id, rest_ty.clone()),
                    _ => set_pattern_types(working_set, item, &item_ty),
                }
            }
        }
        _ => {}
    }
}

fn parse_pattern_in(
    working_set: &mut StateWorkingSet,
    span: Span,
    ctx: PatternContext,
) -> MatchPattern {
    let bytes = working_set.get_span_contents(span);

    // `$x?`, `{...}?`, `[...]?` and `_?`: a field or item that may be missing. A bare word like
    // `foo?` stays a string.
    if bytes.len() > 1
        && bytes.ends_with(b"?")
        && (matches!(bytes[0], b'$' | b'{' | b'[') || bytes == b"_?")
    {
        let inner = parse_pattern_in(working_set, Span::new(span.start, span.end - 1), ctx);
        return MatchPattern {
            pattern: Pattern::Optional {
                pattern: Box::new(inner),
                default: None,
            },
            guard: None,
            span,
        };
    }

    if bytes.starts_with(b"$") {
        // Variable pattern
        parse_variable_pattern(working_set, span, ctx)
    } else if bytes.starts_with(b"{") {
        // Record pattern
        parse_record_pattern(working_set, span, ctx)
    } else if bytes.starts_with(b"[") {
        // List pattern
        parse_list_pattern(working_set, span, ctx)
    } else if bytes == b"_" {
        MatchPattern {
            pattern: Pattern::IgnoreValue,
            guard: None,
            span,
        }
    } else {
        // Literal / expression pattern (including parenthesized const expressions).
        // `parse_value` already routes `(` through `parse_paren_expr`.
        parse_value_pattern(working_set, span)
    }
}

/// Parse a non-structural match pattern and const-evaluate it to [`Pattern::Value`].
///
/// Parenthesized expressions, bare literals, and ranges all go through the normal value
/// parser, then `eval_constant`. Non-constant expressions become parse errors rather than
/// silently failing to match at runtime.
fn parse_value_pattern(working_set: &mut StateWorkingSet, span: Span) -> MatchPattern {
    let expr = parse_value(working_set, span, &SyntaxShape::Any, None);

    // Avoid stacking a const-eval error on top of an existing parse failure.
    if matches!(expr.expr, Expr::Garbage) {
        return garbage(span);
    }

    match eval_constant(working_set, &expr) {
        Ok(val) => MatchPattern {
            pattern: Pattern::Value(val),
            guard: None,
            span,
        },
        Err(e) => {
            working_set.error(e.wrap(working_set, span));
            garbage(span)
        }
    }
}

fn parse_variable_pattern_helper(
    working_set: &mut StateWorkingSet,
    span: Span,
    ctx: PatternContext,
) -> Option<VarId> {
    let bytes = working_set.get_span_contents(span);

    if !is_variable(bytes) {
        return None;
    }
    let mutable = match ctx {
        PatternContext::Match => {
            if let Some(var_id) = working_set.find_variable_in_current_frame(bytes) {
                return Some(var_id);
            }
            false
        }
        PatternContext::Binding { mutable } => mutable,
    };
    let name = bytes.to_vec();
    ensure_not_reserved_variable_name(working_set, &name, span);
    Some(working_set.add_variable(name, span, Type::Any, mutable))
}

fn parse_variable_pattern(
    working_set: &mut StateWorkingSet,
    span: Span,
    ctx: PatternContext,
) -> MatchPattern {
    if let Some(var_id) = parse_variable_pattern_helper(working_set, span, ctx) {
        MatchPattern {
            pattern: Pattern::Variable(var_id),
            guard: None,
            span,
        }
    } else {
        working_set.error(ParseError::Expected("valid variable name", span));
        garbage(span)
    }
}

/// Give `pattern` the default in `default`, from `pattern = default`, which makes it optional.
///
/// The default must be a constant, like a parameter default, and fit `pattern`.
fn parse_default(
    working_set: &mut StateWorkingSet,
    pattern: MatchPattern,
    equals: Span,
    default: Option<Span>,
) -> MatchPattern {
    let Some(default_span) = default else {
        working_set.error(ParseError::Expected(
            "default value",
            Span::new(equals.end, equals.end),
        ));
        return pattern;
    };
    if matches!(pattern.pattern, Pattern::Optional { .. }) {
        working_set.error(ParseError::LabeledErrorWithHelp {
            error: "A pattern can have `?` or a default, not both".into(),
            label: "already optional".into(),
            help: "Remove the `?`. A default already covers a missing value.".into(),
            span: pattern.span,
        });
        return pattern;
    }
    let Pattern::Value(default) = parse_value_pattern(working_set, default_span).pattern else {
        // `parse_value_pattern` reported why it isn't a constant.
        return pattern;
    };
    if let Err(mismatch) = pattern.check(&default, &mut vec![]) {
        let error = mismatch.into_parse_error();
        working_set.error(error);
    }
    MatchPattern {
        span: Span::new(pattern.span.start, default_span.end),
        pattern: Pattern::Optional {
            pattern: Box::new(pattern),
            default: Some(Box::new(default)),
        },
        guard: None,
    }
}

/// Parse `..$name`, the rest of a list or record pattern.
fn parse_rest_pattern(
    working_set: &mut StateWorkingSet,
    span: Span,
    ctx: PatternContext,
) -> MatchPattern {
    let name_span = Span::new(span.start + 2, span.end);
    if let Some(var_id) = parse_variable_pattern_helper(working_set, name_span, ctx) {
        MatchPattern {
            pattern: Pattern::Rest(var_id),
            guard: None,
            span,
        }
    } else {
        working_set.error(ParseError::Expected("valid variable name", span));
        garbage(span)
    }
}

fn parse_list_pattern(
    working_set: &mut StateWorkingSet,
    span: Span,
    ctx: PatternContext,
) -> MatchPattern {
    let bytes = working_set.get_span_contents(span);

    let mut start = span.start;
    let mut end = span.end;

    if bytes.starts_with(b"[") {
        start += 1;
    }
    if bytes.ends_with(b"]") {
        end -= 1;
    } else {
        let open = ParseError::opener_span(span, 1);
        working_set.error(ParseError::unclosed("]", open, Span::new(end, end)));
    }

    let inner_span = Span::new(start, end);
    let (output, err) = lex_span(working_set, inner_span, &[b'\n', b'\r', b','], &[], true);
    if let Some(err) = err {
        working_set.error(err);
    }

    if let Some(token) = output
        .iter()
        .find(|token| token.contents == TokenContents::Semicolon)
    {
        working_set.error(ParseError::LabeledErrorWithHelp {
            error: "Unexpected semicolon in list pattern".into(),
            label: "not a valid list separator".into(),
            help: "Use commas or whitespace to separate list items.".into(),
            span: token.span,
        });
    }

    let (output, err) = lite_parse(&output, working_set);
    if let Some(err) = err {
        working_set.error(err);
    }

    let mut args = vec![];
    // Like a Rust slice pattern, a list pattern may hold one `..` or `..$name`, at any position.
    let mut has_rest = false;

    if !output.block.is_empty() {
        for command in &output.block[0].commands {
            let mut parts = command.parts.iter().copied().peekable();
            while let Some(part) = parts.next() {
                let contents = working_set.get_span_contents(part);

                let arg = if contents == b".." {
                    MatchPattern {
                        pattern: Pattern::IgnoreRest,
                        guard: None,
                        span: part,
                    }
                } else if contents.starts_with(b"..$") {
                    parse_rest_pattern(working_set, part, ctx)
                } else {
                    let item = parse_pattern_in(working_set, part, ctx);
                    // `pattern = default`
                    match parts.next_if(|next| working_set.get_span_contents(*next) == b"=") {
                        Some(equals) => parse_default(working_set, item, equals, parts.next()),
                        None => item,
                    }
                };

                let is_rest = matches!(arg.pattern, Pattern::IgnoreRest | Pattern::Rest(_));
                if is_rest && has_rest {
                    // An extra `..$name` was still parsed above, so its variable is declared
                    // and the arm body reports only this error, not also "Variable not found".
                    working_set.error(ParseError::LabeledErrorWithHelp {
                        error: "`..` can only be used once per list pattern".into(),
                        label: "extra rest pattern".into(),
                        help: "Keep one `..` or `..$name` and remove the others.".into(),
                        span: part,
                    });
                    args.push(garbage(part));
                } else {
                    has_rest |= is_rest;
                    args.push(arg);
                }
            }
        }
    }

    // Optional items come last, and not with a rest: in `[$a?, $b]` nothing says which item
    // `[1]` fills, and in `[$a?, ..]` both would take up a short list.
    let is_optional = |item: &MatchPattern| matches!(item.pattern, Pattern::Optional { .. });
    if let Some(first_optional) = args.iter().position(is_optional) {
        if has_rest {
            working_set.error(ParseError::LabeledErrorWithHelp {
                error: "A list pattern can't have both `..` and optional items".into(),
                label: "optional item".into(),
                help: "Remove the `?`, or remove the `..` so the list has a fixed length.".into(),
                span: args[first_optional].span,
            });
        } else if let Some(required) = args[first_optional..]
            .iter()
            .find(|item| !is_optional(item))
        {
            working_set.error(ParseError::LabeledErrorWithHelp {
                error: "A required item can't follow an optional one".into(),
                label: "required item after an optional one".into(),
                help: "Move the optional items to the end of the list pattern.".into(),
                span: required.span,
            });
        }
    }

    MatchPattern {
        pattern: Pattern::List(args),
        guard: None,
        span,
    }
}

fn parse_record_pattern(
    working_set: &mut StateWorkingSet,
    span: Span,
    ctx: PatternContext,
) -> MatchPattern {
    let mut bytes = working_set.get_span_contents(span);

    let mut start = span.start;
    let mut end = span.end;

    if bytes.starts_with(b"{") {
        start += 1;
    } else {
        working_set.error(ParseError::Expected("{", Span::new(start, start + 1)));
        bytes = working_set.get_span_contents(span);
    }

    if bytes.ends_with(b"}") {
        end -= 1;
    } else {
        let open = ParseError::opener_span(span, 1);
        working_set.error(ParseError::unclosed("}", open, Span::new(end, end)));
    }

    let inner_span = Span::new(start, end);
    let (tokens, err) = lex_span(
        working_set,
        inner_span,
        &[b'\n', b'\r', b','],
        &[b':'],
        true,
    );
    if let Some(err) = err {
        working_set.error(err);
    }

    let mut output = vec![];
    let mut idx = 0;
    let mut has_rest = false;

    while idx < tokens.len() {
        let bytes = working_set.get_span_contents(tokens[idx].span);
        let (field, pattern) = if bytes == b".." {
            working_set.error(ParseError::LabeledErrorWithHelp {
                error: "A record pattern already ignores the fields it doesn't name".into(),
                label: "not needed".into(),
                help: "Remove the `..`, or write `..$rest` to bind the other fields.".into(),
                span: tokens[idx].span,
            });
            idx += 1;
            continue;
        } else if bytes.starts_with(b"..$") {
            // `..$rest` binds the other fields. It has no field name of its own.
            let pattern = parse_rest_pattern(working_set, tokens[idx].span, ctx);
            if has_rest {
                working_set.error(ParseError::LabeledErrorWithHelp {
                    error: "`..$name` can only be used once per record pattern".into(),
                    label: "extra rest pattern".into(),
                    help: "Keep one `..$name` and remove the others.".into(),
                    span: tokens[idx].span,
                });
            }
            has_rest = true;
            (String::new(), pattern)
        } else if bytes.starts_with(b"$") {
            // `{$a}` is short for `{a: $a}`, and `{$a?}` for `{a: $a?}`.
            let name = &bytes[1..];
            let name = name.strip_suffix(b"?").unwrap_or(name);
            let field = String::from_utf8_lossy(name).to_string();

            let pattern = parse_pattern_in(working_set, tokens[idx].span, ctx);

            (field, pattern)
        } else {
            // A quoted key names the field without its quotes, as in a record literal.
            let (field, err) = unescape_unquote_string(bytes, tokens[idx].span);
            if let Some(err) = err {
                working_set.error(err);
            }

            idx += 1;
            if idx == tokens.len() {
                working_set.error(ParseError::Expected("record", span));
                return garbage(span);
            }
            let colon = working_set.get_span_contents(tokens[idx].span);
            idx += 1;
            if idx == tokens.len() || colon != b":" {
                //FIXME: need better error
                working_set.error(ParseError::Expected("record", span));
                return garbage(span);
            }
            let pattern = parse_pattern_in(working_set, tokens[idx].span, ctx);

            (field, pattern)
        };
        idx += 1;

        // `field: pattern = default` and `$field = default`
        let pattern = match tokens.get(idx) {
            Some(equals) if working_set.get_span_contents(equals.span) == b"=" => {
                let default = tokens.get(idx + 1).map(|token| token.span);
                idx += 2;
                parse_default(working_set, pattern, equals.span, default)
            }
            _ => pattern,
        };

        output.push((field, pattern));
    }

    MatchPattern {
        pattern: Pattern::Record(output),
        guard: None,
        span,
    }
}
