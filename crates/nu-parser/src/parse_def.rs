use crate::{
    known_external::KnownExternal,
    lite_parser::LiteCommand,
    parse_helpers::{SPREAD_OPERATOR, garbage},
    parse_pipelines::redirecting_builtin_error,
    parser::{
        ArgumentParsingLevel, CallKind, ParsedInternalCall, compile_block_with_id, parse_attribute,
        parse_full_signature, parse_internal_call, parse_string,
    },
    type_check::check_block_input_output,
};

use itertools::Itertools;
use nu_protocol::{
    CommandWideCompleter, CustomExample, DeclId, FromValue, ParseError, PositionalArg, Signature,
    Span, Spanned, SyntaxShape, Type, Value,
    ast::{AttributeBlock, Call, Expr, Expression, Pipeline},
    category_from_string,
    engine::{CommandType, StateWorkingSet},
    eval_const::eval_constant,
    shell_error::generic::GenericError,
};

fn rest_param_is_type_annotated(signature_source: &[u8], rest_name: &str) -> bool {
    let mut needle = Vec::with_capacity(rest_name.len() + 3);
    needle.extend_from_slice(SPREAD_OPERATOR);
    needle.extend_from_slice(rest_name.as_bytes());

    if signature_source.len() < needle.len() {
        return false;
    }

    for start in 0..=(signature_source.len() - needle.len()) {
        if signature_source[start..start + needle.len()] != needle {
            continue;
        }

        let mut idx = start + needle.len();
        while idx < signature_source.len() && signature_source[idx].is_ascii_whitespace() {
            idx += 1;
        }

        if idx < signature_source.len() && signature_source[idx] == b':' {
            return true;
        }
    }

    false
}

pub fn parse_def_predecl(working_set: &mut StateWorkingSet, spans: &[Span]) {
    let mut pos = 0;

    let def_type_name = if spans.len() >= 3 {
        let first_word = working_set.get_span_contents(spans[0]);

        if first_word == b"export" {
            pos += 2;
        } else {
            pos += 1;
        }

        working_set.get_span_contents(spans[pos - 1]).to_vec()
    } else {
        return;
    };

    if def_type_name != b"def" && def_type_name != b"extern" {
        return;
    }

    while pos < spans.len() && working_set.get_span_contents(spans[pos]).starts_with(b"-") {
        pos += 1;
    }

    if pos >= spans.len() {
        return;
    }

    let name_pos = pos;

    let Some(name) = parse_string(working_set, spans[name_pos]).as_string() else {
        return;
    };

    if name.contains('#')
        || name.contains('^')
        || name.contains('%')
        || name.parse::<bytesize::ByteSize>().is_ok()
        || name.parse::<f64>().is_ok()
    {
        working_set.error(ParseError::CommandDefNotValid(spans[name_pos]));
        return;
    }

    let mut signature_pos = None;

    while pos < spans.len() {
        if working_set.get_span_contents(spans[pos]).starts_with(b"[")
            || working_set.get_span_contents(spans[pos]).starts_with(b"(")
        {
            signature_pos = Some(pos);
            break;
        }

        pos += 1;
    }

    let Some(signature_pos) = signature_pos else {
        return;
    };

    let mut allow_unknown_args = false;

    for span in spans {
        if working_set.get_span_contents(*span) == b"--wrapped" && def_type_name == b"def" {
            allow_unknown_args = true;
        }
    }

    let starting_error_count = working_set.parse_errors.len();

    working_set.enter_scope();
    let sig = parse_full_signature(
        working_set,
        &spans[signature_pos..],
        def_type_name == b"extern",
    );
    working_set.parse_errors.truncate(starting_error_count);
    working_set.exit_scope();

    let Some(mut signature) = sig.as_signature() else {
        return;
    };

    signature.name = name;

    if allow_unknown_args {
        if let Some(rest) = &mut signature.rest_positional
            && !rest_param_is_type_annotated(
                working_set.get_span_contents(spans[signature_pos]),
                &rest.name,
            )
        {
            rest.shape = SyntaxShape::ExternalArgument;
        }
        signature.allows_unknown_args = true;
    }

    let command_type = if def_type_name == b"extern" {
        CommandType::External
    } else {
        CommandType::Custom
    };

    let decl = signature.predeclare_with_command_type(command_type);

    if working_set.add_predecl(decl).is_some() {
        working_set.error(ParseError::DuplicateCommandDef(spans[name_pos]));
    }
}

pub fn parse_for(working_set: &mut StateWorkingSet, lite_command: &LiteCommand) -> Expression {
    let spans = &lite_command.parts;
    if working_set.get_span_contents(spans[0]) != b"for" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for 'for' function".into(),
            Span::concat(spans),
        ));
        return garbage(working_set, spans[0]);
    }
    if let Some(redirection) = lite_command.redirection.as_ref() {
        working_set.error(redirecting_builtin_error("for", redirection));
        return garbage(working_set, spans[0]);
    }

    let Some(decl_id) = working_set.find_decl(b"for") else {
        working_set.error(ParseError::UnknownState(
            "internal error: for declaration not found".into(),
            Span::concat(spans),
        ));
        return garbage(working_set, spans[0]);
    };

    let starting_error_count = working_set.parse_errors.len();
    working_set.enter_scope();
    let ParsedInternalCall {
        call,
        output,
        call_kind,
    } = parse_internal_call(
        working_set,
        spans[0],
        &spans[1..],
        decl_id,
        ArgumentParsingLevel::Full,
        None,
    );

    if working_set
        .parse_errors
        .get(starting_error_count..)
        .is_none_or(|new_errors| {
            new_errors
                .iter()
                .all(|e| !matches!(e, ParseError::Unclosed(token, _) if *token == "}"))
        })
    {
        working_set.exit_scope();
    }

    let call_span = Span::concat(spans);
    let decl = working_set.get_decl(decl_id);
    let sig = decl.signature();

    if call_kind != CallKind::Valid {
        return Expression::new(working_set, Expr::Call(call), call_span, output);
    }

    let [var_decl, iteration_expr, block_expr] = call
        .positional_iter()
        .next_array()
        .expect("for call already checked");

    if let Expression {
        expr: Expr::Block(block_id) | Expr::RowCondition(block_id),
        ..
    } = block_expr
    {
        let block = working_set.get_block_mut(*block_id);

        *block.signature = sig;
    };

    let var_type = match iteration_expr.ty.clone() {
        Type::List(x) => *x,
        Type::Table(x) => Type::Record(x),
        Type::Range => Type::Number,
        x => x,
    };

    if let (Some(var_id), Some(block_id)) = (var_decl.as_var(), block_expr.as_block()) {
        working_set.set_variable_type(var_id, var_type.clone());

        let block = working_set.get_block_mut(block_id);
        block.signature.required_positional.insert(
            0,
            PositionalArg {
                name: String::new(),
                desc: String::new(),
                shape: var_type.to_shape(),
                var_id: Some(var_id),
                default_value: None,
                completion: None,
            },
        );
    }

    Expression::new(working_set, Expr::Call(call), call_span, Type::Nothing)
}

pub fn parse_attribute_block(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
) -> Pipeline {
    let attributes = lite_command
        .attribute_commands()
        .map(|cmd| parse_attribute(working_set, &cmd).0)
        .collect::<Vec<_>>();

    let last_attr_span = attributes
        .last()
        .expect("Attribute block must contain at least one attribute")
        .expr
        .span;

    working_set.error(ParseError::AttributeRequiresDefinition(last_attr_span));
    let cmd_span = if lite_command.command_parts().is_empty() {
        last_attr_span.past()
    } else {
        Span::concat(lite_command.command_parts())
    };
    let cmd_expr = garbage(working_set, cmd_span);
    let ty = cmd_expr.ty.clone();

    let attr_block_span = Span::merge_many(
        attributes
            .first()
            .map(|x| x.expr.span)
            .into_iter()
            .chain(Some(cmd_span)),
    );

    Pipeline::from_vec(vec![Expression::new(
        working_set,
        Expr::AttributeBlock(AttributeBlock {
            attributes,
            item: Box::new(cmd_expr),
        }),
        attr_block_span,
        ty,
    )])
}

pub fn parse_def(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
    module_name: Option<&[u8]>,
) -> (Pipeline, Option<(Vec<u8>, DeclId)>) {
    let mut attributes = vec![];
    let mut attribute_vals = vec![];

    for attr_cmd in lite_command.attribute_commands() {
        let (attr, name) = parse_attribute(working_set, &attr_cmd);
        if let Some(name) = name {
            let val = eval_constant(working_set, &attr.expr);
            match val {
                Ok(val) => attribute_vals.push((name, val)),
                Err(e) => working_set.error(e.wrap(working_set, attr.expr.span)),
            }
        }
        attributes.push(attr);
    }

    let (expr, decl) = parse_def_inner(working_set, attribute_vals, lite_command, module_name);

    let ty = expr.ty.clone();

    let attr_block_span = Span::merge_many(
        attributes
            .first()
            .map(|x| x.expr.span)
            .into_iter()
            .chain(Some(expr.span)),
    );

    let expr = if attributes.is_empty() {
        expr
    } else {
        Expression::new(
            working_set,
            Expr::AttributeBlock(AttributeBlock {
                attributes,
                item: Box::new(expr),
            }),
            attr_block_span,
            ty,
        )
    };

    (Pipeline::from_vec(vec![expr]), decl)
}

pub fn parse_extern(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
    module_name: Option<&[u8]>,
) -> Pipeline {
    let mut attributes = vec![];
    let mut attribute_vals = vec![];

    for attr_cmd in lite_command.attribute_commands() {
        let (attr, name) = parse_attribute(working_set, &attr_cmd);
        if let Some(name) = name {
            let val = eval_constant(working_set, &attr.expr);
            match val {
                Ok(val) => attribute_vals.push((name, val)),
                Err(e) => working_set.error(e.wrap(working_set, attr.expr.span)),
            }
        }
        attributes.push(attr);
    }

    let expr = parse_extern_inner(working_set, attribute_vals, lite_command, module_name);

    let ty = expr.ty.clone();

    let attr_block_span = Span::merge_many(
        attributes
            .first()
            .map(|x| x.expr.span)
            .into_iter()
            .chain(Some(expr.span)),
    );

    let expr = if attributes.is_empty() {
        expr
    } else {
        Expression::new(
            working_set,
            Expr::AttributeBlock(AttributeBlock {
                attributes,
                item: Box::new(expr),
            }),
            attr_block_span,
            ty,
        )
    };

    Pipeline::from_vec(vec![expr])
}

fn parse_def_inner(
    working_set: &mut StateWorkingSet,
    attributes: Vec<(String, Value)>,
    lite_command: &LiteCommand,
    module_name: Option<&[u8]>,
) -> (Expression, Option<(Vec<u8>, DeclId)>) {
    let spans = lite_command.command_parts();

    let (desc, extra_desc) = working_set.build_desc(&lite_command.comments);
    let garbage_result =
        |working_set: &mut StateWorkingSet<'_>| (garbage(working_set, Span::concat(spans)), None);

    let (name_span, split_id) =
        if spans.len() > 1 && working_set.get_span_contents(spans[0]) == b"export" {
            (spans[1], 2)
        } else {
            (spans[0], 1)
        };

    let def_call = working_set.get_span_contents(name_span);
    if def_call != b"def" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for def function".into(),
            Span::concat(spans),
        ));
        return garbage_result(working_set);
    }
    if let Some(redirection) = lite_command.redirection.as_ref() {
        working_set.error(redirecting_builtin_error("def", redirection));
        return garbage_result(working_set);
    }

    let Some(decl_id) = working_set.permanent_state.find_decl(def_call, &[]) else {
        working_set.error(ParseError::UnknownState(
            "internal error: def declaration not found".into(),
            Span::concat(spans),
        ));
        return garbage_result(working_set);
    };

    working_set.enter_scope();
    let (command_spans, rest_spans) = spans.split_at(split_id);

    let mut decl_name_span = None;

    for span in rest_spans {
        if !working_set.get_span_contents(*span).starts_with(b"-") {
            decl_name_span = Some(*span);
            break;
        }
    }

    if let Some(name_span) = decl_name_span
        && let Some(err) = detect_params_in_name(working_set, name_span, decl_id)
    {
        working_set.error(err);
        return garbage_result(working_set);
    }

    let starting_error_count = working_set.parse_errors.len();
    let ParsedInternalCall {
        call,
        output,
        call_kind,
    } = parse_internal_call(
        working_set,
        Span::concat(command_spans),
        rest_spans,
        decl_id,
        ArgumentParsingLevel::Full,
        None,
    );

    if working_set
        .parse_errors
        .get(starting_error_count..)
        .is_none_or(|new_errors| {
            new_errors
                .iter()
                .all(|e| !matches!(e, ParseError::Unclosed(token, _) if *token == "}"))
        })
    {
        working_set.exit_scope();
    }

    let call_span = Span::concat(spans);
    let decl = working_set.get_decl(decl_id);
    let sig = decl.signature();

    match call.positional_iter().nth(2) {
        Some(Expression {
            expr: Expr::Closure(block_id),
            ..
        }) => {
            compile_block_with_id(working_set, *block_id);
            *working_set.get_block_mut(*block_id).signature = sig.clone();
        }
        Some(arg) => working_set.error(ParseError::Expected(
            "definition body closure { ... }",
            arg.span,
        )),
        None => (),
    }

    if call_kind != CallKind::Valid {
        return (
            Expression::new(working_set, Expr::Call(call), call_span, output),
            None,
        );
    }

    let Ok(has_env) = has_flag_const(working_set, &call, "env") else {
        return garbage_result(working_set);
    };
    let Ok(has_wrapped) = has_flag_const(working_set, &call, "wrapped") else {
        return garbage_result(working_set);
    };

    let [name_expr, sig_expr, block_expr] = call
        .positional_iter()
        .next_array()
        .expect("def call already checked");

    let Some(name) = name_expr.as_string() else {
        working_set.error(ParseError::UnknownState(
            "Could not get string from string expression".into(),
            name_expr.span,
        ));
        return garbage_result(working_set);
    };

    if let Some(mod_name) = module_name
        && name.as_bytes() == mod_name
    {
        let name_expr_span = name_expr.span;

        working_set.error(ParseError::NamedAsModule(
            "command".to_string(),
            name,
            "main".to_string(),
            name_expr_span,
        ));
        return (
            Expression::new(working_set, Expr::Call(call), call_span, Type::Any),
            None,
        );
    }

    let mut result = None;

    if let (Some(mut signature), Some(block_id)) = (sig_expr.as_signature(), block_expr.as_block())
    {
        if has_wrapped {
            let Some(rest) = signature.rest_positional.as_mut() else {
                working_set.error(ParseError::MissingPositional(
                    "...rest-like positional argument".to_string(),
                    name_expr.span,
                    "def --wrapped must have a ...rest-like positional argument. \
                            Add '...rest: string' to the command's signature."
                        .to_string(),
                ));

                return (
                    Expression::new(working_set, Expr::Call(call), call_span, Type::Any),
                    result,
                );
            };

            if !rest_param_is_type_annotated(
                working_set.get_span_contents(sig_expr.span),
                &rest.name,
            ) {
                rest.shape = SyntaxShape::ExternalArgument;
            }

            if let Some(var_id) = rest.var_id {
                let rest_var = &working_set.get_variable(var_id);

                if rest_var.ty != Type::Any && rest_var.ty != Type::List(Box::new(Type::String)) {
                    working_set.error(ParseError::TypeMismatchHelp(
                        Type::List(Box::new(Type::String)),
                        rest_var.ty.clone(),
                        rest_var.declaration_span,
                        format!(
                            "...rest-like positional argument used in 'def --wrapped' supports only strings. \
                                Change the type annotation of ...{} to 'string'.",
                            &rest.name
                        ),
                    ));

                    return (
                        Expression::new(working_set, Expr::Call(call), call_span, Type::Any),
                        result,
                    );
                }
            }
        }

        if let Some(decl_id) = working_set.find_predecl(name.as_bytes()) {
            signature.name.clone_from(&name);
            if !has_wrapped {
                *signature = signature.add_help();
            }
            signature.description = desc;
            signature.extra_description = extra_desc;
            signature.allows_unknown_args = has_wrapped;

            let (attribute_vals, examples) =
                handle_special_attributes(attributes, working_set, &mut signature);

            let declaration = working_set.get_decl_mut(decl_id);

            *declaration = signature
                .clone()
                .into_block_command(block_id, attribute_vals, examples);

            let block = working_set.get_block_mut(block_id);
            block.signature = signature;
            block.redirect_env = has_env;

            if block.signature.input_output_types.is_empty() {
                block
                    .signature
                    .input_output_types
                    .push((Type::Any, Type::Any));
            }

            let block = working_set.get_block(block_id);

            let typecheck_errors = check_block_input_output(working_set, block);

            working_set
                .parse_errors
                .extend_from_slice(&typecheck_errors);

            result = Some((name.as_bytes().to_vec(), decl_id));
        } else {
            working_set.error(ParseError::InternalError(
                "Predeclaration failed to add declaration".into(),
                name_expr.span,
            ));
        };
    }

    working_set.merge_predecl(name.as_bytes());

    (
        Expression::new(working_set, Expr::Call(call), call_span, Type::Any),
        result,
    )
}

fn parse_extern_inner(
    working_set: &mut StateWorkingSet,
    attributes: Vec<(String, Value)>,
    lite_command: &LiteCommand,
    module_name: Option<&[u8]>,
) -> Expression {
    let spans = lite_command.command_parts();

    let (description, extra_description) = working_set.build_desc(&lite_command.comments);

    let (name_span, split_id) =
        if spans.len() > 1 && (working_set.get_span_contents(spans[0]) == b"export") {
            (spans[1], 2)
        } else {
            (spans[0], 1)
        };

    let extern_call = working_set.get_span_contents(name_span);
    if extern_call != b"extern" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for extern command".into(),
            Span::concat(spans),
        ));
        return garbage(working_set, Span::concat(spans));
    }
    if let Some(redirection) = lite_command.redirection.as_ref() {
        working_set.error(redirecting_builtin_error("extern", redirection));
        return garbage(working_set, Span::concat(spans));
    }

    let (call, call_span) = match working_set.permanent().find_decl(extern_call, &[]) {
        None => {
            working_set.error(ParseError::UnknownState(
                "internal error: def declaration not found".into(),
                Span::concat(spans),
            ));
            return garbage(working_set, Span::concat(spans));
        }
        Some(decl_id) => {
            working_set.enter_scope();

            let (command_spans, rest_spans) = spans.split_at(split_id);

            if let Some(name_span) = rest_spans.first()
                && let Some(err) = detect_params_in_name(working_set, *name_span, decl_id)
            {
                working_set.error(err);
                return garbage(working_set, Span::concat(spans));
            }

            let ParsedInternalCall { call, .. } = parse_internal_call(
                working_set,
                Span::concat(command_spans),
                rest_spans,
                decl_id,
                ArgumentParsingLevel::Full,
                None,
            );
            working_set.exit_scope();

            let call_span = Span::concat(spans);

            (call, call_span)
        }
    };

    let (name_and_sig_exprs, body_expr) = {
        let mut positional_iter = call.positional_iter();
        (positional_iter.next_array::<2>(), positional_iter.next())
    };

    if let Some([name_expr, sig]) = name_and_sig_exprs {
        if let (Some(name), Some(mut signature)) = (&name_expr.as_string(), sig.as_signature()) {
            if let Some(mod_name) = module_name
                && name.as_bytes() == mod_name
            {
                let name_expr_span = name_expr.span;
                working_set.error(ParseError::NamedAsModule(
                    "known external".to_string(),
                    name.clone(),
                    "main".to_string(),
                    name_expr_span,
                ));
                return Expression::new(working_set, Expr::Call(call), call_span, Type::Any);
            }

            if let Some(decl_id) = working_set.find_predecl(name.as_bytes()) {
                let external_name = if let Some(mod_name) = module_name {
                    if name.as_bytes() == b"main" {
                        String::from_utf8_lossy(mod_name).to_string()
                    } else {
                        name.clone()
                    }
                } else {
                    name.clone()
                };

                signature.name = external_name;
                signature.description = description;
                signature.extra_description = extra_description;
                signature.allows_unknown_args = true;

                let (attribute_vals, examples) =
                    handle_special_attributes(attributes, working_set, &mut signature);

                let declaration = working_set.get_decl_mut(decl_id);

                if let Some(block_id) = body_expr.and_then(|x| x.as_block()) {
                    if signature.rest_positional.is_none() {
                        working_set.error(ParseError::InternalError(
                            "Extern block must have a rest positional argument".into(),
                            name_expr.span,
                        ));
                    } else {
                        *declaration = signature.clone().into_block_command(
                            block_id,
                            attribute_vals,
                            examples,
                        );

                        working_set.get_block_mut(block_id).signature = signature;
                    }
                } else {
                    if signature.rest_positional.is_none() {
                        *signature = signature.rest(
                            "args",
                            SyntaxShape::ExternalArgument,
                            "All other arguments to the command.",
                        );
                    }

                    let decl = KnownExternal {
                        signature,
                        attributes: attribute_vals,
                        examples,
                        span: call_span,
                    };

                    *declaration = Box::new(decl);
                }
            } else {
                working_set.error(ParseError::InternalError(
                    "Predeclaration failed to add declaration".into(),
                    spans[split_id],
                ));
            };
        }
        if let Some(name) = name_expr.as_string() {
            working_set.merge_predecl(name.as_bytes());
        } else {
            working_set.error(ParseError::UnknownState(
                "Could not get string from string expression".into(),
                name_expr.span,
            ));
        }
    }

    Expression::new(working_set, Expr::Call(call), call_span, Type::Any)
}

fn handle_special_attributes(
    attributes: Vec<(String, Value)>,
    working_set: &mut StateWorkingSet<'_>,
    signature: &mut Signature,
) -> (Vec<(String, Value)>, Vec<CustomExample>) {
    let mut attribute_vals = vec![];
    let mut examples = vec![];
    let mut search_terms = vec![];
    let mut category = String::new();

    for (name, value) in attributes {
        let val_span = value.span();
        match name.as_str() {
            "example" => match CustomExample::from_value(value) {
                Ok(example) => examples.push(example),
                Err(_) => {
                    let e = nu_protocol::ShellError::Generic(
                        GenericError::new(
                            "nu::shell::invalid_example",
                            "Value couldn't be converted to an example",
                            val_span,
                        )
                        .with_help("Is `attr example` shadowed?"),
                    );
                    working_set.error(e.wrap(working_set, val_span));
                }
            },
            "search-terms" => match <Vec<String>>::from_value(value) {
                Ok(mut terms) => {
                    search_terms.append(&mut terms);
                }
                Err(_) => {
                    let e = nu_protocol::ShellError::Generic(
                        GenericError::new(
                            "nu::shell::invalid_search_terms",
                            "Value couldn't be converted to search-terms",
                            val_span,
                        )
                        .with_help("Is `attr search-terms` shadowed?"),
                    );
                    working_set.error(e.wrap(working_set, val_span));
                }
            },
            "category" => match <String>::from_value(value) {
                Ok(term) => {
                    category.push_str(&term);
                }
                Err(_) => {
                    let e = nu_protocol::ShellError::Generic(
                        GenericError::new(
                            "nu::shell::invalid_category",
                            "Value couldn't be converted to category",
                            val_span,
                        )
                        .with_help("Is `attr category` shadowed?"),
                    );
                    working_set.error(e.wrap(working_set, val_span));
                }
            },
            "complete" => match <Spanned<String>>::from_value(value) {
                Ok(Spanned { item, span }) => {
                    if let Some(decl) = working_set.find_decl(item.as_bytes()) {
                        signature.complete = Some(CommandWideCompleter::Command(decl));
                    } else {
                        working_set.error(ParseError::UnknownCommand(span));
                    }
                }
                Err(_) => {
                    let e = nu_protocol::ShellError::Generic(
                        GenericError::new(
                            "nu::shell::invalid_completer",
                            "Value couldn't be converted to a completer",
                            val_span,
                        )
                        .with_help("Is `attr complete` shadowed?"),
                    );
                    working_set.error(e.wrap(working_set, val_span));
                }
            },
            "complete external" => match value {
                nu_protocol::Value::Nothing { .. } => {
                    signature.complete = Some(CommandWideCompleter::External);
                }
                _ => {
                    let e = nu_protocol::ShellError::Generic(
                        GenericError::new(
                            "nu::shell::invalid_completer",
                            "This attribute shouldn't return anything",
                            val_span,
                        )
                        .with_help("Is `attr complete` shadowed?"),
                    );
                    working_set.error(e.wrap(working_set, val_span));
                }
            },
            _ => {
                attribute_vals.push((name, value));
            }
        }
    }

    signature.search_terms = search_terms;
    signature.category = category_from_string(&category);

    (attribute_vals, examples)
}

fn detect_params_in_name(
    working_set: &StateWorkingSet,
    name_span: Span,
    decl_id: DeclId,
) -> Option<ParseError> {
    let name = working_set.get_span_contents(name_span);
    for (offset, char) in name.iter().enumerate() {
        if *char == b'[' || *char == b'(' {
            return Some(ParseError::LabeledErrorWithHelp {
                error: "no space between name and parameters".into(),
                label: "expected space".into(),
                help: format!(
                    "consider adding a space between the `{}` command's name and its parameters",
                    working_set.get_decl(decl_id).name()
                ),
                span: Span::new(offset + name_span.start - 1, offset + name_span.start - 1),
            });
        }
    }

    None
}

pub(crate) fn has_flag_const(
    working_set: &mut StateWorkingSet,
    call: &Call,
    name: &str,
) -> Result<bool, ()> {
    call.has_flag_const(working_set, name).map_err(|err| {
        working_set.error(err.wrap(working_set, call.span()));
    })
}
