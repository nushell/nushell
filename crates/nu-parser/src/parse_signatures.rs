#![allow(clippy::byte_char_slices)]

use crate::parse_captures_compile::compile_block;
use crate::parse_expressions::{parse_closure_expression, parse_math_expression, parse_value};
use crate::parse_literals::parse_full_cell_path;
use crate::{
    Token, TokenContents,
    lex::lex_signature,
    parse_helpers::{SPREAD_OPERATOR, garbage, is_variable},
    parse_shape_specs::{parse_completer, parse_shape_name, parse_type},
    type_check::type_compatible,
};
use log::trace;
use nu_protocol::{
    Flag, ParseError, PositionalArg, Signature, Span, SyntaxShape, Type, TypeSet, Value, VarId,
    ast::*, engine::StateWorkingSet, eval_const::eval_constant,
};
use std::{collections::HashSet, sync::Arc};

pub fn parse_import_pattern<'a>(
    working_set: &mut StateWorkingSet,
    mut arg_iter: impl Iterator<Item = &'a Expression>,
    spans: &[Span],
) -> Expression {
    let Some(head_expr) = arg_iter.next() else {
        working_set.error(ParseError::WrongImportPattern(
            "needs at least one component of import pattern".to_string(),
            Span::concat(spans),
        ));
        return garbage(working_set, Span::concat(spans));
    };

    let (maybe_module_id, head_name) = match eval_constant(working_set, head_expr) {
        Ok(Value::Nothing { .. }) => {
            return Expression::new(
                working_set,
                Expr::Nothing,
                Span::concat(spans),
                Type::Nothing,
            );
        }
        Ok(val) => match val.coerce_into_string() {
            Ok(s) => (working_set.find_module(s.as_bytes()), s.into_bytes()),
            Err(err) => {
                working_set.error(err.wrap(working_set, Span::concat(spans)));
                return garbage(working_set, Span::concat(spans));
            }
        },
        Err(err) => {
            working_set.error(err.wrap(working_set, Span::concat(spans)));
            return garbage(working_set, Span::concat(spans));
        }
    };

    let mut import_pattern = ImportPattern {
        head: ImportPatternHead {
            name: head_name,
            id: maybe_module_id,
            span: head_expr.span,
        },
        members: vec![],
        hidden: HashSet::new(),
        constants: vec![],
    };

    let mut leaf_member_expr: Option<(&str, Span)> = None;

    // TODO: box pattern syntax is experimental @rust v1.89.0
    let handle_list_items =
        |items: &Vec<ListItem>,
         span,
         working_set: &mut StateWorkingSet<'_>,
         import_pattern: &mut ImportPattern,
         leaf_member_expr: &mut Option<(&str, Span)>| {
            let mut output = vec![];

            for item in items.iter() {
                match item {
                    ListItem::Item(expr) => {
                        if let Some(name) = expr.as_string() {
                            output.push((name.as_bytes().to_vec(), expr.span));
                        }
                    }
                    ListItem::Spread(_, spread) => {
                        working_set.error(ParseError::WrongImportPattern(
                            "cannot spread in an import pattern".into(),
                            spread.span,
                        ))
                    }
                }
            }

            import_pattern
                .members
                .push(ImportPatternMember::List { names: output });

            *leaf_member_expr = Some(("list", span));
        };

    for tail_expr in arg_iter {
        if let Some((what, prev_span)) = leaf_member_expr {
            working_set.error(ParseError::WrongImportPattern(
                format!("{what} member can be only at the end of an import pattern"),
                prev_span,
            ));
            return Expression::new(
                working_set,
                Expr::ImportPattern(Box::new(import_pattern)),
                prev_span,
                Type::List(Box::new(Type::String)),
            );
        }

        match &tail_expr.expr {
            Expr::String(name) => {
                let span = tail_expr.span;
                if name == "*" {
                    import_pattern
                        .members
                        .push(ImportPatternMember::Glob { span });

                    leaf_member_expr = Some(("glob", span));
                } else {
                    import_pattern.members.push(ImportPatternMember::Name {
                        name: name.as_bytes().to_vec(),
                        span,
                    });
                }
            }
            Expr::FullCellPath(fcp) => {
                if let Expr::List(items) = &fcp.head.expr {
                    handle_list_items(
                        items,
                        fcp.head.span,
                        working_set,
                        &mut import_pattern,
                        &mut leaf_member_expr,
                    );
                }
            }
            Expr::List(items) => {
                handle_list_items(
                    items,
                    tail_expr.span,
                    working_set,
                    &mut import_pattern,
                    &mut leaf_member_expr,
                );
            }
            _ => {
                working_set.error(ParseError::WrongImportPattern(
                    "Wrong type of import pattern, only String and List<String> are allowed."
                        .into(),
                    tail_expr.span,
                ));
            }
        };
    }

    Expression::new(
        working_set,
        Expr::ImportPattern(Box::new(import_pattern)),
        Span::concat(&spans[1..]),
        Type::List(Box::new(Type::String)),
    )
}

pub fn parse_var_with_opt_type(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    spans_idx: &mut usize,
    mutable: bool,
) -> (Expression, Option<Type>) {
    let name_span = spans[*spans_idx];
    let bytes = working_set.get_span_contents(name_span);

    if bytes.contains(&b' ')
        || bytes.contains(&b'"')
        || bytes.contains(&b'\'')
        || bytes.contains(&b'`')
    {
        working_set.error(ParseError::VariableNotValid(spans[*spans_idx]));
        return (garbage(working_set, spans[*spans_idx]), None);
    }

    if bytes.ends_with(b":") {
        let name_span = Span::new(name_span.start, name_span.end - 1);
        let var_name = bytes[0..(bytes.len() - 1)].to_vec();

        // We end with colon, so the next span should be the type
        if *spans_idx + 1 < spans.len() {
            *spans_idx += 1;
            // signature like record<a: int b: int> is broken into multiple spans due to
            // whitespaces. Collect the rest into one span and work on it
            let full_span = Span::concat(&spans[*spans_idx..]);
            let type_bytes = working_set.get_span_contents(full_span).to_vec();

            let (tokens, parse_error) =
                lex_signature(&type_bytes, full_span.start, &[], &[b','], true);

            if let Some(parse_error) = parse_error {
                working_set.error(parse_error);
            }

            let ty = parse_type(working_set, &type_bytes, tokens[0].span);
            *spans_idx = spans.len() - 1;

            if !is_variable(&var_name) {
                working_set.error(ParseError::Expected(
                    "valid variable name",
                    spans[*spans_idx - 1],
                ));
                return (garbage(working_set, spans[*spans_idx - 1]), None);
            }

            ensure_not_reserved_variable_name(working_set, &var_name, name_span);

            let id = working_set.add_variable(var_name, spans[*spans_idx - 1], ty.clone(), mutable);

            (
                Expression::new(working_set, Expr::VarDecl(id), name_span, ty.clone()),
                Some(ty),
            )
        } else {
            if !is_variable(&var_name) {
                working_set.error(ParseError::Expected(
                    "valid variable name",
                    spans[*spans_idx],
                ));
                return (garbage(working_set, spans[*spans_idx]), None);
            }

            ensure_not_reserved_variable_name(working_set, &var_name, name_span);

            let id = working_set.add_variable(var_name, spans[*spans_idx], Type::Any, mutable);

            working_set.error(ParseError::MissingType(spans[*spans_idx]));
            (
                Expression::new(working_set, Expr::VarDecl(id), spans[*spans_idx], Type::Any),
                None,
            )
        }
    } else {
        let var_name = bytes.to_vec();

        if !is_variable(&var_name) {
            working_set.error(ParseError::Expected(
                "valid variable name",
                spans[*spans_idx],
            ));
            return (garbage(working_set, spans[*spans_idx]), None);
        }

        ensure_not_reserved_variable_name(working_set, &var_name, name_span);

        let id = working_set.add_variable(
            var_name,
            Span::concat(&spans[*spans_idx..*spans_idx + 1]),
            Type::Any,
            mutable,
        );

        (
            Expression::new(working_set, Expr::VarDecl(id), spans[*spans_idx], Type::Any),
            None,
        )
    }
}

const RESERVED_VARIABLE_NAMES: [&[u8]; 3] = [b"in", b"nu", b"env"];

pub(crate) fn ensure_not_reserved_variable_name(
    working_set: &mut StateWorkingSet,
    name: &[u8],
    span: Span,
) {
    let var_name = name.strip_prefix(b"$").unwrap_or(name);

    if RESERVED_VARIABLE_NAMES.contains(&var_name) {
        working_set.error(ParseError::NameIsBuiltinVar(
            String::from_utf8_lossy(var_name).to_string(),
            span,
        ))
    }
}

pub fn expand_to_cell_path(
    working_set: &mut StateWorkingSet,
    expression: &mut Expression,
    var_id: VarId,
) {
    trace!("parsing: expanding to cell path");
    if let Expression {
        expr: Expr::String(_),
        span,
        ..
    } = expression
    {
        // Re-parse the string as if it were a cell-path
        let new_expression = parse_full_cell_path(working_set, Some(var_id), *span);

        *expression = new_expression;
    }

    if let Expression {
        expr: Expr::UnaryNot(inner),
        ..
    } = expression
    {
        expand_to_cell_path(working_set, inner, var_id);
    }
}

pub fn parse_input_output_types(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
) -> Vec<(Type, Type)> {
    let mut full_span = Span::concat(spans);

    let mut bytes = working_set.get_span_contents(full_span);

    if bytes.starts_with(b"[") {
        bytes = &bytes[1..];
        full_span.start += 1;
    }

    if bytes.ends_with(b"]") {
        bytes = &bytes[..(bytes.len() - 1)];
        full_span.end -= 1;
    }

    let (tokens, parse_error) =
        lex_signature(bytes, full_span.start, &[b'\n', b'\r', b','], &[], true);

    if let Some(parse_error) = parse_error {
        working_set.error(parse_error);
    }

    let mut output = vec![];

    let mut idx = 0;
    while idx < tokens.len() {
        let type_bytes = working_set.get_span_contents(tokens[idx].span).to_vec();
        let input_type = parse_type(working_set, &type_bytes, tokens[idx].span);

        idx += 1;
        if idx >= tokens.len() {
            working_set.error(ParseError::Expected(
                "arrow (->)",
                Span::new(tokens[idx - 1].span.end, tokens[idx - 1].span.end),
            ));
            break;
        }

        let arrow = working_set.get_span_contents(tokens[idx].span);
        if arrow != b"->" {
            working_set.error(ParseError::Expected("arrow (->)", tokens[idx].span));
        }

        idx += 1;
        if idx >= tokens.len() {
            working_set.error(ParseError::MissingType(Span::new(
                tokens[idx - 1].span.end,
                tokens[idx - 1].span.end,
            )));
            break;
        }

        let type_bytes = working_set.get_span_contents(tokens[idx].span).to_vec();
        let output_type = parse_type(working_set, &type_bytes, tokens[idx].span);

        output.push((input_type, output_type));

        idx += 1;
    }

    output
}

pub fn parse_full_signature(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    is_external: bool,
) -> Expression {
    match spans.len() {
        // This case should never happen. It corresponds to declarations like `def foo {}`,
        // which should throw a 'Missing required positional argument.' before getting to this point
        0 => {
            working_set.error(ParseError::InternalError(
                "failed to catch missing positional arguments".to_string(),
                Span::concat(spans),
            ));
            garbage(working_set, Span::concat(spans))
        }

        // e.g. `[ b"[foo: string]" ]`
        1 => parse_signature(working_set, spans[0], is_external),

        // This case is needed to distinguish between e.g.
        // `[ b"[]", b"{ true }" ]` vs `[ b"[]:", b"int" ]`
        2 if working_set.get_span_contents(spans[1]).starts_with(b"{") => {
            parse_signature(working_set, spans[0], is_external)
        }

        // This should handle every other case, e.g.
        // `[ b"[]:", b"int" ]`
        // `[ b"[]", b":", b"int" ]`
        // `[ b"[]", b":", b"int", b"->", b"bool" ]`
        _ => {
            let (mut arg_signature, input_output_types_pos) =
                if working_set.get_span_contents(spans[0]).ends_with(b":") {
                    (
                        parse_signature(
                            working_set,
                            Span::new(spans[0].start, spans[0].end.saturating_sub(1)),
                            is_external,
                        ),
                        1,
                    )
                } else if working_set.get_span_contents(spans[1]) == b":" {
                    (parse_signature(working_set, spans[0], is_external), 2)
                } else {
                    // This should be an error case, but we call parse_signature anyway
                    // so it can handle the various possible errors
                    // e.g. `[ b"[]", b"int" ]` or `[
                    working_set.error(ParseError::Expected(
                        "colon (:) before type signature",
                        Span::concat(&spans[1..]),
                    ));
                    // (garbage(working_set, Span::concat(spans)), 1)

                    (parse_signature(working_set, spans[0], is_external), 1)
                };

            let input_output_types =
                parse_input_output_types(working_set, &spans[input_output_types_pos..]);

            if let Expression {
                expr: Expr::Signature(sig),
                span: expr_span,
                ..
            } = &mut arg_signature
            {
                sig.input_output_types = input_output_types;
                expr_span.end = Span::concat(&spans[input_output_types_pos..]).end;
            }
            arg_signature
        }
    }
}

pub fn parse_row_condition(working_set: &mut StateWorkingSet, spans: &[Span]) -> Expression {
    let pos = spans.first().map(|s| s.start).unwrap_or(0);
    let span = Span::concat(spans);

    // Try parsing the expression as a closure first
    {
        let err_count = working_set.parse_errors.len();
        let expression = parse_closure_expression(working_set, &SyntaxShape::Any, span);
        if working_set.parse_errors.len() == err_count
            && let Expr::Closure(block_id) = expression.expr
        {
            // Successfully parsed a closure
            return Expression::new(working_set, Expr::RowCondition(block_id), span, Type::Bool);
        }
        // Couldn't parse a closure, roll back the errors and try parsing it as a row condition
        working_set.parse_errors.truncate(err_count);
    }
    // The following code can still parse closures (and it did so before the preceding shortcut).
    // It is very unlikely to happen, but should be kept in mind.

    // New scope in case where there's already a variable named `$it`
    working_set.enter_scope();
    let var_id = working_set.add_variable(b"$it".to_vec(), Span::new(pos, pos), Type::Any, false);
    let expression = parse_math_expression(working_set, spans, Some(var_id));

    let block_id = match expression.expr {
        Expr::Block(block_id) => block_id,
        Expr::Closure(block_id) => block_id,
        Expr::FullCellPath(ref box_fcp) if box_fcp.head.as_var().is_some_and(|id| id != var_id) => {
            let mut expression = expression;
            expression.ty = Type::Any;
            working_set.exit_scope();
            return expression;
        }
        Expr::Var(arg_var_id) if arg_var_id != var_id => {
            let mut expression = expression;
            expression.ty = Type::Any;
            working_set.exit_scope();
            return expression;
        }
        _ => {
            // We have an expression, check that it's compatible with bool
            if !type_compatible(&Type::Bool, &expression.ty) {
                working_set.error(ParseError::TypeMismatch(
                    Type::Bool,
                    expression.ty.clone(),
                    expression.span,
                ));
                working_set.exit_scope();
                return Expression::garbage(working_set, expression.span);
            }

            // Convert this expression into a block.
            let mut block = Block::new();
            let mut pipeline = Pipeline::new();
            pipeline.elements.push(PipelineElement {
                pipe: None,
                expr: expression,
                redirection: None,
            });

            block.pipelines.push(pipeline);

            block.signature.required_positional.push(PositionalArg {
                name: "$it".into(),
                desc: "row condition".into(),
                shape: SyntaxShape::Any,
                var_id: Some(var_id),
                default_value: None,
                completion: None,
            });

            compile_block(working_set, &mut block);

            working_set.add_block(Arc::new(block))
        }
    };
    working_set.exit_scope();

    Expression::new(working_set, Expr::RowCondition(block_id), span, Type::Bool)
}

pub fn parse_signature(
    working_set: &mut StateWorkingSet,
    span: Span,
    is_external: bool,
) -> Expression {
    let bytes = working_set.get_span_contents(span);

    let mut start = span.start;
    let mut end = span.end;

    let mut has_paren = false;

    if bytes.starts_with(b"[") {
        start += 1;
    } else if bytes.starts_with(b"(") {
        has_paren = true;
        start += 1;
    } else {
        working_set.error(ParseError::Expected("[ or (", Span::new(start, start + 1)));
        return garbage(working_set, span);
    }

    if (has_paren && bytes.ends_with(b")")) || (!has_paren && bytes.ends_with(b"]")) {
        end -= 1;
    } else {
        working_set.error(ParseError::Unclosed("] or )", Span::new(end, end)));
    }

    let sig = parse_signature_helper(working_set, Span::new(start, end), is_external);

    Expression::new(working_set, Expr::Signature(sig), span, Type::Any)
}

pub fn parse_signature_helper(
    working_set: &mut StateWorkingSet,
    span: Span,
    is_external: bool,
) -> Box<Signature> {
    enum ParseMode {
        Arg,
        AfterCommaArg,
        Type,
        AfterType,
        DefaultValue,
    }

    #[derive(Debug)]
    enum Arg {
        Positional {
            arg: PositionalArg,
            required: bool,
            type_annotated: bool,
        },
        RestPositional(PositionalArg),
        Flag {
            flag: Flag,
            type_annotated: bool,
        },
    }

    let source = working_set.get_span_contents(span);

    let (output, err) = lex_signature(
        source,
        span.start,
        &[b'\n', b'\r'],
        &[b':', b'=', b','],
        false,
    );
    if let Some(err) = err {
        working_set.error(err);
    }

    let mut args: Vec<Arg> = vec![];
    let mut parse_mode = ParseMode::Arg;
    // Track variables whose name→VarId mappings have not yet been inserted
    // into the overlay scope
    //
    // We defer all insertions until the entire signature is parsed so that
    // default value expressions always resolve to outer scope variables,
    // not to sibling parameters
    //
    // See #15306
    let mut pending_scope_inserts: Vec<(Vec<u8>, VarId)> = vec![];

    for (index, token) in output.iter().enumerate() {
        let last_token = index == output.len() - 1;

        match token {
            Token {
                contents: TokenContents::Item | TokenContents::AssignmentOperator,
                span,
            } => {
                let span = *span;
                let contents = working_set.get_span_contents(span).to_vec();

                // The : symbol separates types
                if contents == b":" {
                    match parse_mode {
                        ParseMode::Arg if last_token => working_set
                            .error(ParseError::Expected("type", Span::new(span.end, span.end))),
                        ParseMode::Arg => {
                            parse_mode = ParseMode::Type;
                        }
                        ParseMode::AfterCommaArg | ParseMode::AfterType => {
                            working_set.error(ParseError::Expected("parameter or flag", span));
                        }
                        ParseMode::Type | ParseMode::DefaultValue => {
                            // We're seeing two types for the same thing for some reason, error
                            working_set.error(ParseError::Expected("type", span));
                        }
                    }
                }
                // The = symbol separates a variable from its default value
                else if contents == b"=" {
                    match parse_mode {
                        ParseMode::Arg | ParseMode::AfterType if last_token => working_set.error(
                            ParseError::Expected("default value", Span::new(span.end, span.end)),
                        ),
                        ParseMode::Arg | ParseMode::AfterType => {
                            parse_mode = ParseMode::DefaultValue;
                        }
                        ParseMode::Type => {
                            working_set.error(ParseError::Expected("type", span));
                        }
                        ParseMode::AfterCommaArg => {
                            working_set.error(ParseError::Expected("parameter or flag", span));
                        }
                        ParseMode::DefaultValue => {
                            // We're seeing two default values for some reason, error
                            working_set.error(ParseError::Expected("default value", span));
                        }
                    }
                }
                // The , symbol separates params only
                else if contents == b"," {
                    match parse_mode {
                        ParseMode::Arg | ParseMode::AfterType => {
                            parse_mode = ParseMode::AfterCommaArg
                        }
                        ParseMode::AfterCommaArg => {
                            working_set.error(ParseError::Expected("parameter or flag", span));
                        }
                        ParseMode::Type => {
                            working_set.error(ParseError::Expected("type", span));
                        }
                        ParseMode::DefaultValue => {
                            working_set.error(ParseError::Expected("default value", span));
                        }
                    }
                } else {
                    let mut check_and_add_variable =
                        |working_set: &mut StateWorkingSet,
                         var_name: Vec<u8>,
                         ty: Type,
                         span: Span| {
                            if is_external {
                                None
                            } else {
                                ensure_not_reserved_variable_name(working_set, &var_name, span);
                                let var_id =
                                    working_set.add_variable_without_scope(span, ty, false);
                                pending_scope_inserts.push((var_name, var_id));
                                Some(var_id)
                            }
                        };

                    match parse_mode {
                        ParseMode::Arg | ParseMode::AfterCommaArg | ParseMode::AfterType => {
                            // Long flag with optional short form following with no whitespace, e.g. --output, --age(-a)
                            if contents.starts_with(b"--") && contents.len() > 2 {
                                // Split the long flag from the short flag with the ( character as delimiter.
                                // The trailing ) is removed further down.
                                let flags: Vec<_> = contents.split(|x| x == &b'(').collect();

                                let long = String::from_utf8_lossy(&flags[0][2..]).to_string();
                                let mut variable_name = flags[0][2..].to_vec();
                                // Replace the '-' in a variable name with '_'
                                for byte in variable_name.iter_mut() {
                                    if *byte == b'-' {
                                        *byte = b'_';
                                    }
                                }

                                if !is_variable(&variable_name) {
                                    working_set.error(ParseError::Expected(
                                        "valid variable name for this long flag",
                                        span,
                                    ))
                                }

                                let var_id = check_and_add_variable(
                                    working_set,
                                    variable_name,
                                    Type::Bool,
                                    span,
                                );

                                // If there's no short flag, exit now. Otherwise, parse it.
                                if flags.len() == 1 {
                                    args.push(Arg::Flag {
                                        flag: Flag {
                                            arg: None,
                                            desc: String::new(),
                                            long,
                                            short: None,
                                            required: false,
                                            var_id,
                                            default_value: None,
                                            completion: None,
                                        },
                                        type_annotated: false,
                                    });
                                } else if flags.len() >= 3 {
                                    working_set.error(ParseError::Expected(
                                        "only one short flag alternative",
                                        span,
                                    ));
                                } else {
                                    let short_flag = &flags[1];
                                    let short_flag = if !short_flag.starts_with(b"-")
                                        || !short_flag.ends_with(b")")
                                    {
                                        working_set.error(ParseError::Expected(
                                            "short flag alternative for the long flag",
                                            span,
                                        ));
                                        short_flag
                                    } else {
                                        // Obtain the flag's name by removing the starting - and trailing )
                                        &short_flag[1..(short_flag.len() - 1)]
                                    };
                                    // Note that it is currently possible to make a short flag with non-alphanumeric characters,
                                    // like -).

                                    let short_flag =
                                        String::from_utf8_lossy(short_flag).to_string();
                                    let chars: Vec<char> = short_flag.chars().collect();

                                    if chars.len() == 1 {
                                        args.push(Arg::Flag {
                                            flag: Flag {
                                                arg: None,
                                                desc: String::new(),
                                                long,
                                                short: Some(chars[0]),
                                                required: false,
                                                var_id,
                                                default_value: None,
                                                completion: None,
                                            },
                                            type_annotated: false,
                                        });
                                    } else {
                                        working_set.error(ParseError::Expected("short flag", span));
                                    }
                                }
                                parse_mode = ParseMode::Arg;
                            }
                            // Mandatory short flag, e.g. -e (must be one character)
                            else if contents.starts_with(b"-") && contents.len() > 1 {
                                let short_flag = &contents[1..];
                                let short_flag = String::from_utf8_lossy(short_flag).to_string();
                                let chars: Vec<char> = short_flag.chars().collect();

                                if chars.len() > 1 {
                                    working_set.error(ParseError::Expected("short flag", span));
                                }

                                let mut encoded_var_name = [0u8; 4];
                                let len = chars[0].encode_utf8(&mut encoded_var_name).len();
                                let variable_name = encoded_var_name[0..len].to_vec();

                                if !is_variable(&variable_name) {
                                    working_set.error(ParseError::Expected(
                                        "valid variable name for this short flag",
                                        span,
                                    ))
                                }

                                let var_id = check_and_add_variable(
                                    working_set,
                                    variable_name,
                                    Type::Bool,
                                    span,
                                );

                                args.push(Arg::Flag {
                                    flag: Flag {
                                        arg: None,
                                        desc: String::new(),
                                        long: String::new(),
                                        short: Some(chars[0]),
                                        required: false,
                                        var_id,
                                        default_value: None,
                                        completion: None,
                                    },
                                    type_annotated: false,
                                });
                                parse_mode = ParseMode::Arg;
                            }
                            // Short flag alias for long flag, e.g. --b (-a)
                            // This is the same as the short flag in --b(-a)
                            else if let Some(short_flag) = contents.strip_prefix(b"(-") {
                                if let ParseMode::AfterCommaArg = parse_mode {
                                    working_set
                                        .error(ParseError::Expected("parameter or flag", span));
                                }

                                let short_flag = if !short_flag.ends_with(b")") {
                                    working_set.error(ParseError::Expected("short flag", span));
                                    short_flag
                                } else {
                                    &short_flag[..(short_flag.len() - 1)]
                                };

                                let short_flag = String::from_utf8_lossy(short_flag).to_string();
                                let chars: Vec<char> = short_flag.chars().collect();

                                if chars.len() == 1 {
                                    match args.last_mut() {
                                        Some(Arg::Flag { flag, .. }) => {
                                            if flag.short.is_some() {
                                                working_set.error(ParseError::Expected(
                                                    "one short flag",
                                                    span,
                                                ));
                                            } else {
                                                flag.short = Some(chars[0]);
                                            }
                                        }
                                        _ => {
                                            working_set
                                                .error(ParseError::Expected("unknown flag", span));
                                        }
                                    }
                                } else {
                                    working_set.error(ParseError::Expected("short flag", span));
                                }
                            }
                            // Positional arg, optional
                            else if let Some(optional_param) = contents.strip_suffix(b"?") {
                                let name = String::from_utf8_lossy(optional_param).to_string();

                                if !is_variable(optional_param) {
                                    working_set.error(ParseError::Expected(
                                        "valid variable name for this optional parameter",
                                        span,
                                    ))
                                }

                                let var_id = check_and_add_variable(
                                    working_set,
                                    optional_param.to_vec(),
                                    Type::Any,
                                    span,
                                );

                                args.push(Arg::Positional {
                                    arg: PositionalArg {
                                        desc: String::new(),
                                        name,
                                        shape: SyntaxShape::Any,
                                        var_id,
                                        default_value: None,
                                        completion: None,
                                    },
                                    required: false,
                                    type_annotated: false,
                                });
                                parse_mode = ParseMode::Arg;
                            }
                            // Rest param
                            else if let Some(contents) = contents.strip_prefix(SPREAD_OPERATOR) {
                                let name = String::from_utf8_lossy(contents).to_string();
                                let contents_vec: Vec<u8> = contents.to_vec();

                                if !is_variable(&contents_vec) {
                                    working_set.error(ParseError::Expected(
                                        "valid variable name for this rest parameter",
                                        span,
                                    ))
                                }

                                let var_id = check_and_add_variable(
                                    working_set,
                                    contents_vec,
                                    Type::Any,
                                    span,
                                );

                                args.push(Arg::RestPositional(PositionalArg {
                                    desc: String::new(),
                                    name,
                                    shape: SyntaxShape::Any,
                                    var_id,
                                    default_value: None,
                                    completion: None,
                                }));
                                parse_mode = ParseMode::Arg;
                            }
                            // Normal param
                            else {
                                let name = String::from_utf8_lossy(&contents).to_string();
                                let contents_vec = contents.to_vec();

                                if !is_variable(&contents_vec) {
                                    working_set.error(ParseError::Expected(
                                        "valid variable name for this parameter",
                                        span,
                                    ))
                                }

                                let var_id = check_and_add_variable(
                                    working_set,
                                    contents_vec,
                                    Type::Any,
                                    span,
                                );

                                // Positional arg, required
                                args.push(Arg::Positional {
                                    arg: PositionalArg {
                                        desc: String::new(),
                                        name,
                                        shape: SyntaxShape::Any,
                                        var_id,
                                        default_value: None,
                                        completion: None,
                                    },
                                    required: true,
                                    type_annotated: false,
                                });
                                parse_mode = ParseMode::Arg;
                            }
                        }
                        ParseMode::Type => {
                            if let Some(last) = args.last_mut() {
                                let (syntax_shape, completer) = contents
                                    .iter()
                                    .position(|b| *b == b'@')
                                    .and_then(|idx| {
                                        let (shape, completer) = contents.split_at_checked(idx)?;
                                        let (shape_span, completer_span) = span.split_at(idx)?;

                                        let completer = completer.strip_prefix(b"@")?;
                                        let (_, completer_span) = completer_span.split_at(1)?;

                                        Some((
                                            parse_shape_name(working_set, shape, shape_span),
                                            parse_completer(working_set, completer, completer_span),
                                        ))
                                    })
                                    .unwrap_or_else(|| {
                                        (parse_shape_name(working_set, &contents, span), None)
                                    });

                                //TODO check if we're replacing a custom parameter already
                                match last {
                                    Arg::Positional {
                                        arg:
                                            PositionalArg {
                                                shape,
                                                var_id,
                                                completion,
                                                ..
                                            },
                                        required: _,
                                        type_annotated,
                                    } => {
                                        if !is_external {
                                            working_set.set_variable_type(
                                                var_id.expect(
                                                    "internal error: all custom parameters must have \
                                                    var_ids",
                                                ),
                                                syntax_shape.to_type(),
                                            );
                                        }
                                        *completion = completer;
                                        *shape = syntax_shape;
                                        *type_annotated = true;
                                    }
                                    Arg::RestPositional(PositionalArg {
                                        shape,
                                        var_id,
                                        completion,
                                        ..
                                    }) => {
                                        if !is_external {
                                            working_set.set_variable_type(
                                                var_id.expect(
                                                    "internal error: all custom parameters must have \
                                                    var_ids",
                                                ),
                                                Type::List(Box::new(syntax_shape.to_type())),
                                            );
                                        }
                                        *completion = completer;
                                        *shape = syntax_shape;
                                    }
                                    Arg::Flag {
                                        flag:
                                            Flag {
                                                arg,
                                                var_id,
                                                completion,
                                                ..
                                            },
                                        type_annotated,
                                    } => {
                                        if !is_external {
                                            working_set.set_variable_type(var_id.expect("internal error: all custom parameters must have var_ids"), syntax_shape.to_type());
                                        }
                                        if syntax_shape == SyntaxShape::Boolean {
                                            working_set.error(ParseError::LabeledError(
                                                "Type annotations are not allowed for boolean switches.".to_string(),
                                                "Remove the `: bool` type annotation.".to_string(),
                                                span,
                                            ));
                                        }
                                        *completion = completer;
                                        *arg = Some(syntax_shape);
                                        *type_annotated = true;
                                    }
                                }
                            }
                            parse_mode = ParseMode::AfterType;
                        }
                        ParseMode::DefaultValue => {
                            if !is_external && let Some(last) = args.last_mut() {
                                let shape = match last {
                                    Arg::Positional { arg, .. } => arg.shape.clone(),
                                    Arg::RestPositional(arg) => arg.shape.clone(),
                                    Arg::Flag { flag, .. } => {
                                        flag.arg.clone().unwrap_or(SyntaxShape::Any)
                                    }
                                };

                                let expression = parse_value(working_set, span, &shape);

                                //TODO check if we're replacing a custom parameter already
                                match last {
                                    Arg::Positional {
                                        arg:
                                            PositionalArg {
                                                shape,
                                                var_id,
                                                default_value,
                                                ..
                                            },
                                        required,
                                        type_annotated,
                                    } => {
                                        let var_id = var_id.expect("internal error: all custom parameters must have var_ids");
                                        let var_type = &working_set.get_variable(var_id).ty;
                                        if var_type == &Type::Any && !*type_annotated {
                                            working_set
                                                .set_variable_type(var_id, expression.ty.clone());
                                        }

                                        *default_value = if let Ok(constant) =
                                            eval_constant(working_set, &expression)
                                        {
                                            Some(constant)
                                        } else {
                                            working_set.error(ParseError::NonConstantDefaultValue(
                                                expression.span,
                                            ));
                                            None
                                        };

                                        if !*type_annotated {
                                            *shape = expression.ty.to_shape();
                                        }
                                        *required = false;
                                    }
                                    Arg::RestPositional(..) => {
                                        working_set.error(ParseError::AssignmentMismatch(
                                            "Rest parameter was given a default value".into(),
                                            "can't have default value".into(),
                                            expression.span,
                                        ))
                                    }
                                    Arg::Flag {
                                        flag:
                                            Flag {
                                                arg,
                                                var_id,
                                                default_value,
                                                ..
                                            },
                                        type_annotated,
                                    } => {
                                        let expression_span = expression.span;

                                        *default_value = if let Ok(value) =
                                            eval_constant(working_set, &expression)
                                        {
                                            Some(value)
                                        } else {
                                            working_set.error(ParseError::NonConstantDefaultValue(
                                                expression_span,
                                            ));
                                            None
                                        };

                                        let var_id = var_id.expect("internal error: all custom parameters must have var_ids");
                                        let expression_ty = expression.ty.clone();

                                        // Flags without type annotations are present/not-present
                                        // switches *except* when they have a default value
                                        // assigned. In that case they are regular flags and take
                                        // on the type of their default value.
                                        if !*type_annotated {
                                            *arg = Some(expression_ty.to_shape());
                                            working_set.set_variable_type(var_id, expression_ty);
                                        }
                                    }
                                }
                            }
                            parse_mode = ParseMode::Arg;
                        }
                    }
                }
            }
            Token {
                contents: TokenContents::Comment,
                span,
            } => {
                let contents = working_set.get_span_contents(Span::new(span.start + 1, span.end));

                let mut contents = String::from_utf8_lossy(contents).to_string();
                contents = contents.trim().into();

                if let Some(last) = args.last_mut() {
                    match last {
                        Arg::Flag { flag, .. } => {
                            if !flag.desc.is_empty() {
                                flag.desc.push('\n');
                            }
                            flag.desc.push_str(&contents);
                        }
                        Arg::Positional {
                            arg: positional, ..
                        } => {
                            if !positional.desc.is_empty() {
                                positional.desc.push('\n');
                            }
                            positional.desc.push_str(&contents);
                        }
                        Arg::RestPositional(positional) => {
                            if !positional.desc.is_empty() {
                                positional.desc.push('\n');
                            }
                            positional.desc.push_str(&contents);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    for (name, var_id) in pending_scope_inserts {
        working_set.insert_variable_into_scope(name, var_id);
    }

    let mut sig = Signature::new(String::new());

    for arg in args {
        match arg {
            Arg::Positional {
                arg: positional,
                required,
                ..
            } => {
                if required {
                    if !sig.optional_positional.is_empty() {
                        working_set.error(ParseError::RequiredAfterOptional(
                            positional.name.clone(),
                            span,
                        ))
                    }
                    sig.required_positional.push(positional)
                } else {
                    // optional parameters can be `null`
                    // their type within the block should reflect that
                    if positional.default_value.is_none()
                        && let Some(var_id) = positional.var_id
                    {
                        let ty = working_set
                            .get_variable(var_id)
                            .ty
                            .clone()
                            .union(Type::Nothing);
                        working_set.set_variable_type(var_id, ty);
                    };
                    sig.optional_positional.push(positional)
                }
            }
            Arg::Flag {
                flag,
                type_annotated,
                ..
            } => {
                if type_annotated
                    && flag.default_value.is_none()
                    && let Some(var_id) = flag.var_id
                {
                    let ty = working_set
                        .get_variable(var_id)
                        .ty
                        .clone()
                        .union(Type::Nothing);
                    working_set.set_variable_type(var_id, ty);
                }
                sig.named.push(flag)
            }
            Arg::RestPositional(positional) => {
                if positional.name.is_empty() {
                    working_set.error(ParseError::RestNeedsName(span))
                } else if sig.rest_positional.is_none() {
                    sig.rest_positional = Some(PositionalArg {
                        name: positional.name,
                        ..positional
                    })
                } else {
                    // Too many rest params
                    working_set.error(ParseError::MultipleRestParams(span))
                }
            }
        }
    }

    Box::new(sig)
}
