use crate::{
    exportable::Exportable,
    parse_block,
    parser::{
        CallKind, compile_block, compile_block_with_id, parse_attribute, parse_redirection,
        redirecting_builtin_error,
    },
    type_check::{check_block_input_output, type_compatible},
};

use log::trace;
use nu_path::canonicalize_with;
use nu_path::is_windows_device_path;
use nu_protocol::{
    Alias, BlockId, CommandWideCompleter, CustomExample, DeclId, FromValue, Module, ModuleId,
    ParseError, PositionalArg, ResolvedImportPattern, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value, VarId,
    ast::{
        Argument, AttributeBlock, Block, Call, Expr, Expression, ImportPattern, ImportPatternHead,
        ImportPatternMember, Pipeline, PipelineElement,
    },
    category_from_string,
    engine::{DEFAULT_OVERLAY_NAME, StateWorkingSet},
    eval_const::eval_constant,
    parser_path::ParserPath,
};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

pub const LIB_DIRS_VAR: &str = "NU_LIB_DIRS";
#[cfg(feature = "plugin")]
pub const PLUGIN_DIRS_VAR: &str = "NU_PLUGIN_DIRS";

use crate::{
    Token, TokenContents, is_math_expression_like,
    known_external::KnownExternal,
    lex,
    lite_parser::{LiteCommand, lite_parse},
    parser::{
        ParsedInternalCall, garbage, garbage_pipeline, parse, parse_call, parse_expression,
        parse_full_signature, parse_import_pattern, parse_internal_call, parse_string, parse_value,
        parse_var_with_opt_type, trim_quotes,
    },
    unescape_unquote_string,
};

/// These parser keywords can be aliased
pub const ALIASABLE_PARSER_KEYWORDS: &[&[u8]] = &[
    b"if",
    b"match",
    b"try",
    b"overlay",
    b"overlay hide",
    b"overlay new",
    b"overlay use",
];

pub const RESERVED_VARIABLE_NAMES: [&str; 3] = ["in", "nu", "env"];

/// These parser keywords cannot be aliased (either not possible, or support not yet added)
pub const UNALIASABLE_PARSER_KEYWORDS: &[&[u8]] = &[
    b"alias",
    b"const",
    b"def",
    b"extern",
    b"module",
    b"use",
    b"export",
    b"export alias",
    b"export const",
    b"export def",
    b"export extern",
    b"export module",
    b"export use",
    b"for",
    b"loop",
    b"while",
    b"return",
    b"break",
    b"continue",
    b"let",
    b"mut",
    b"hide",
    b"export-env",
    b"source-env",
    b"source",
    b"where",
    b"plugin use",
];

/// Check whether spans start with a parser keyword that can be aliased
pub fn is_unaliasable_parser_keyword(working_set: &StateWorkingSet, spans: &[Span]) -> bool {
    // try two words
    if let (Some(&span1), Some(&span2)) = (spans.first(), spans.get(1)) {
        let cmd_name = working_set.get_span_contents(Span::append(span1, span2));
        return UNALIASABLE_PARSER_KEYWORDS.contains(&cmd_name);
    }

    // try one word
    if let Some(&span1) = spans.first() {
        let cmd_name = working_set.get_span_contents(span1);
        UNALIASABLE_PARSER_KEYWORDS.contains(&cmd_name)
    } else {
        false
    }
}

/// This is a new more compact method of calling parse_xxx() functions without repeating the
/// parse_call() in each function. Remaining keywords can be moved here.
pub fn parse_keyword(working_set: &mut StateWorkingSet, lite_command: &LiteCommand) -> Pipeline {
    let orig_parse_errors_len = working_set.parse_errors.len();

    let call_expr = parse_call(working_set, &lite_command.parts, lite_command.parts[0]);

    // If an error occurred, don't invoke the keyword-specific functionality
    if working_set.parse_errors.len() > orig_parse_errors_len {
        return Pipeline::from_vec(vec![call_expr]);
    }

    if let Expression {
        expr: Expr::Call(call),
        ..
    } = call_expr.clone()
    {
        // Apply parse keyword side effects
        let cmd = working_set.get_decl(call.decl_id);
        // check help flag first.
        if call.named_iter().any(|(flag, _, _)| flag.item == "help") {
            let call_span = call.span();
            return Pipeline::from_vec(vec![Expression::new(
                working_set,
                Expr::Call(call),
                call_span,
                Type::Any,
            )]);
        }

        match cmd.name() {
            "overlay hide" => parse_overlay_hide(working_set, call),
            "overlay new" => parse_overlay_new(working_set, call),
            "overlay use" => parse_overlay_use(working_set, call),
            #[cfg(feature = "plugin")]
            "plugin use" => parse_plugin_use(working_set, call),
            _ => Pipeline::from_vec(vec![call_expr]),
        }
    } else {
        Pipeline::from_vec(vec![call_expr])
    }
}

pub fn parse_def_predecl(working_set: &mut StateWorkingSet, spans: &[Span]) {
    let mut pos = 0;

    let def_type_name = if spans.len() >= 3 {
        // definition can't have only two spans, minimum is 3, e.g., 'extern spam []'
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

    // Now, pos should point at the next span after the def-like call.
    // Skip all potential flags, like --env, --wrapped or --help:
    while pos < spans.len() && working_set.get_span_contents(spans[pos]).starts_with(b"-") {
        pos += 1;
    }

    if pos >= spans.len() {
        // This can happen if the call ends with a flag, e.g., 'def --help'
        return;
    }

    // Now, pos should point at the command name.
    let name_pos = pos;

    let Some(name) = parse_string(working_set, spans[name_pos]).as_string() else {
        return;
    };

    if name.contains('#')
        || name.contains('^')
        || name.parse::<bytesize::ByteSize>().is_ok()
        || name.parse::<f64>().is_ok()
    {
        working_set.error(ParseError::CommandDefNotValid(spans[name_pos]));
        return;
    }

    // Find signature
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
    // FIXME: because parse_signature will update the scope with the variables it sees
    // we end up parsing the signature twice per def. The first time is during the predecl
    // so that we can see the types that are part of the signature, which we need for parsing.
    // The second time is when we actually parse the body itworking_set.
    // We can't reuse the first time because the variables that are created during parse_signature
    // are lost when we exit the scope below.
    let sig = parse_full_signature(working_set, &spans[signature_pos..]);
    working_set.parse_errors.truncate(starting_error_count);
    working_set.exit_scope();

    let Some(mut signature) = sig.as_signature() else {
        return;
    };

    signature.name = name;

    if allow_unknown_args {
        signature.allows_unknown_args = true;
    }

    let decl = signature.predeclare();

    if working_set.add_predecl(decl).is_some() {
        working_set.error(ParseError::DuplicateCommandDef(spans[name_pos]));
    }
}

pub fn parse_for(working_set: &mut StateWorkingSet, lite_command: &LiteCommand) -> Expression {
    let spans = &lite_command.parts;
    // Checking that the function is used with the correct name
    // Maybe this is not necessary but it is a sanity check
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

    // Parsing the spans and checking that they match the register signature
    // Using a parsed call makes more sense than checking for how many spans are in the call
    // Also, by creating a call, it can be checked if it matches the declaration signature
    let (call, call_span) = match working_set.find_decl(b"for") {
        None => {
            working_set.error(ParseError::UnknownState(
                "internal error: for declaration not found".into(),
                Span::concat(spans),
            ));
            return garbage(working_set, spans[0]);
        }
        Some(decl_id) => {
            let starting_error_count = working_set.parse_errors.len();
            working_set.enter_scope();
            let ParsedInternalCall {
                call,
                output,
                call_kind,
            } = parse_internal_call(working_set, spans[0], &spans[1..], decl_id);

            if working_set
                .parse_errors
                .get(starting_error_count..)
                .is_none_or(|new_errors| {
                    new_errors
                        .iter()
                        .all(|e| !matches!(e, ParseError::Unclosed(token, _) if token == "}"))
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

            // Let's get our block and make sure it has the right signature
            if let Some(
                Expression {
                    expr: Expr::Block(block_id),
                    ..
                }
                | Expression {
                    expr: Expr::RowCondition(block_id),
                    ..
                },
            ) = call.positional_nth(2)
            {
                {
                    let block = working_set.get_block_mut(*block_id);

                    block.signature = Box::new(sig);
                }
            }

            (call, call_span)
        }
    };

    // All positional arguments must be in the call positional vector by this point
    let var_decl = call.positional_nth(0).expect("for call already checked");
    let iteration_expr = call.positional_nth(1).expect("for call already checked");
    let block = call.positional_nth(2).expect("for call already checked");

    let iteration_expr_ty = iteration_expr.ty.clone();

    // Figure out the type of the variable the `for` uses for iteration
    let var_type = match iteration_expr_ty {
        Type::List(x) => *x,
        Type::Table(x) => Type::Record(x),
        Type::Range => Type::Number, // Range elements can be int or float
        x => x,
    };

    if let (Some(var_id), Some(block_id)) = (&var_decl.as_var(), block.as_block()) {
        working_set.set_variable_type(*var_id, var_type.clone());

        let block = working_set.get_block_mut(block_id);

        block.signature.required_positional.insert(
            0,
            PositionalArg {
                name: String::new(),
                desc: String::new(),
                shape: var_type.to_shape(),
                var_id: Some(*var_id),
                default_value: None,
                completion: None,
            },
        );
    }

    Expression::new(working_set, Expr::Call(call), call_span, Type::Nothing)
}

/// If `name` is a keyword, emit an error.
fn verify_not_reserved_variable_name(working_set: &mut StateWorkingSet, name: &str, span: Span) {
    if RESERVED_VARIABLE_NAMES.contains(&name) {
        working_set.error(ParseError::NameIsBuiltinVar(name.to_string(), span))
    }
}

// This is meant for parsing attribute blocks without an accompanying `def` or `extern`. It's
// necessary to provide consistent syntax highlighting, completions, and helpful errors
//
// There is no need to run the const evaluation here
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

// Returns also the parsed command name and ID
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

// Returns also the parsed command name and ID
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

    // Checking that the function is used with the correct name
    // Maybe this is not necessary but it is a sanity check
    // Note: "export def" is treated the same as "def"

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

    // Parsing the spans and checking that they match the register signature
    // Using a parsed call makes more sense than checking for how many spans are in the call
    // Also, by creating a call, it can be checked if it matches the declaration signature
    //
    // NOTE: Here we only search for `def` in the permanent state,
    // since recursively redefining `def` is dangerous,
    // see https://github.com/nushell/nushell/issues/16586
    let (call, call_span) = match working_set.permanent_state.find_decl(def_call, &[]) {
        None => {
            working_set.error(ParseError::UnknownState(
                "internal error: def declaration not found".into(),
                Span::concat(spans),
            ));
            return garbage_result(working_set);
        }
        Some(decl_id) => {
            working_set.enter_scope();
            let (command_spans, rest_spans) = spans.split_at(split_id);

            // Find the first span that is not a flag
            let mut decl_name_span = None;

            for span in rest_spans {
                if !working_set.get_span_contents(*span).starts_with(b"-") {
                    decl_name_span = Some(*span);
                    break;
                }
            }

            if let Some(name_span) = decl_name_span {
                // Check whether name contains [] or () -- possible missing space error
                if let Some(err) = detect_params_in_name(working_set, name_span, decl_id) {
                    working_set.error(err);
                    return garbage_result(working_set);
                }
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
            );

            if working_set
                .parse_errors
                .get(starting_error_count..)
                .is_none_or(|new_errors| {
                    new_errors
                        .iter()
                        .all(|e| !matches!(e, ParseError::Unclosed(token, _) if token == "}"))
                })
            {
                working_set.exit_scope();
            }

            let call_span = Span::concat(spans);
            let decl = working_set.get_decl(decl_id);
            let sig = decl.signature();

            // Let's get our block and make sure it has the right signature
            if let Some(arg) = call.positional_nth(2) {
                match arg {
                    Expression {
                        expr: Expr::Closure(block_id),
                        ..
                    } => {
                        // Custom command bodies' are compiled eagerly
                        // 1.  `module`s are not compiled, since they aren't ran/don't have any
                        //     executable code. So `def`s inside modules have to be compiled by
                        //     themselves.
                        // 2.  `def` calls in scripts/runnable code don't *run* any code either,
                        //     they are handled completely by the parser.
                        compile_block_with_id(working_set, *block_id);
                        working_set.get_block_mut(*block_id).signature = Box::new(sig.clone());
                    }
                    _ => working_set.error(ParseError::Expected(
                        "definition body closure { ... }",
                        arg.span,
                    )),
                }
            }

            if call_kind != CallKind::Valid {
                return (
                    Expression::new(working_set, Expr::Call(call), call_span, output),
                    None,
                );
            }

            (call, call_span)
        }
    };

    let Ok(has_env) = has_flag_const(working_set, &call, "env") else {
        return garbage_result(working_set);
    };
    let Ok(has_wrapped) = has_flag_const(working_set, &call, "wrapped") else {
        return garbage_result(working_set);
    };

    // All positional arguments must be in the call positional vector by this point
    let name_expr = call.positional_nth(0).expect("def call already checked");
    let sig = call.positional_nth(1).expect("def call already checked");
    let block = call.positional_nth(2).expect("def call already checked");

    let name = if let Some(name) = name_expr.as_string() {
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

        name
    } else {
        working_set.error(ParseError::UnknownState(
            "Could not get string from string expression".into(),
            name_expr.span,
        ));
        return garbage_result(working_set);
    };

    let mut result = None;

    if let (Some(mut signature), Some(block_id)) = (sig.as_signature(), block.as_block()) {
        for arg_name in &signature.required_positional {
            verify_not_reserved_variable_name(working_set, &arg_name.name, sig.span);
        }
        for arg_name in &signature.optional_positional {
            verify_not_reserved_variable_name(working_set, &arg_name.name, sig.span);
        }
        if let Some(arg_name) = &signature.rest_positional {
            verify_not_reserved_variable_name(working_set, &arg_name.name, sig.span);
        }
        for flag_name in &signature.get_names() {
            verify_not_reserved_variable_name(working_set, flag_name, sig.span);
        }

        if has_wrapped {
            if let Some(rest) = &signature.rest_positional {
                if let Some(var_id) = rest.var_id {
                    let rest_var = &working_set.get_variable(var_id);

                    if rest_var.ty != Type::Any && rest_var.ty != Type::List(Box::new(Type::String))
                    {
                        working_set.error(ParseError::TypeMismatchHelp(
                            Type::List(Box::new(Type::String)),
                            rest_var.ty.clone(),
                            rest_var.declaration_span,
                            format!("...rest-like positional argument used in 'def --wrapped' supports only strings. Change the type annotation of ...{} to 'string'.", &rest.name)));

                        return (
                            Expression::new(working_set, Expr::Call(call), call_span, Type::Any),
                            result,
                        );
                    }
                }
            } else {
                working_set.error(ParseError::MissingPositional("...rest-like positional argument".to_string(), name_expr.span, "def --wrapped must have a ...rest-like positional argument. Add '...rest: string' to the command's signature.".to_string()));

                return (
                    Expression::new(working_set, Expr::Call(call), call_span, Type::Any),
                    result,
                );
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

    // It's OK if it returns None: The decl was already merged in previous parse pass.
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

    // Checking that the function is used with the correct name
    // Maybe this is not necessary but it is a sanity check

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

    // Parsing the spans and checking that they match the register signature
    // Using a parsed call makes more sense than checking for how many spans are in the call
    // Also, by creating a call, it can be checked if it matches the declaration signature
    //
    // NOTE: Here we only search for `extern` in the permanent state,
    // since recursively redefining `extern` is dangerous,
    // see https://github.com/nushell/nushell/issues/16586
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
            );
            working_set.exit_scope();

            let call_span = Span::concat(spans);

            (call, call_span)
        }
    };
    let name_expr = call.positional_nth(0);
    let sig = call.positional_nth(1);
    let body = call.positional_nth(2);

    if let (Some(name_expr), Some(sig)) = (name_expr, sig) {
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

                if let Some(block_id) = body.and_then(|x| x.as_block()) {
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
                        // Make sure that a known external takes rest args with ExternalArgument
                        // shape
                        *signature = signature.rest(
                            "args",
                            SyntaxShape::ExternalArgument,
                            "all other arguments to the command",
                        );
                    }

                    let decl = KnownExternal {
                        signature,
                        attributes: attribute_vals,
                        examples,
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
            // It's OK if it returns None: The decl was already merged in previous parse pass.
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
                    let e = ShellError::GenericError {
                        error: "nu::shell::invalid_example".into(),
                        msg: "Value couldn't be converted to an example".into(),
                        span: Some(val_span),
                        help: Some("Is `attr example` shadowed?".into()),
                        inner: vec![],
                    };
                    working_set.error(e.wrap(working_set, val_span));
                }
            },
            "search-terms" => match <Vec<String>>::from_value(value) {
                Ok(mut terms) => {
                    search_terms.append(&mut terms);
                }
                Err(_) => {
                    let e = ShellError::GenericError {
                        error: "nu::shell::invalid_search_terms".into(),
                        msg: "Value couldn't be converted to search-terms".into(),
                        span: Some(val_span),
                        help: Some("Is `attr search-terms` shadowed?".into()),
                        inner: vec![],
                    };
                    working_set.error(e.wrap(working_set, val_span));
                }
            },
            "category" => match <String>::from_value(value) {
                Ok(term) => {
                    category.push_str(&term);
                }
                Err(_) => {
                    let e = ShellError::GenericError {
                        error: "nu::shell::invalid_category".into(),
                        msg: "Value couldn't be converted to category".into(),
                        span: Some(val_span),
                        help: Some("Is `attr category` shadowed?".into()),
                        inner: vec![],
                    };
                    working_set.error(e.wrap(working_set, val_span));
                }
            },
            "complete" => match <Spanned<String>>::from_value(value) {
                Ok(Spanned { item, span }) => {
                    if let Some(decl) = working_set.find_decl(item.as_bytes()) {
                        // TODO: Enforce command signature? Not before settling on a unified
                        // custom completion api
                        signature.complete = Some(CommandWideCompleter::Command(decl));
                    } else {
                        working_set.error(ParseError::UnknownCommand(span));
                    }
                }
                Err(_) => {
                    let e = ShellError::GenericError {
                        error: "nu::shell::invalid_completer".into(),
                        msg: "Value couldn't be converted to a completer".into(),
                        span: Some(val_span),
                        help: Some("Is `attr complete` shadowed?".into()),
                        inner: vec![],
                    };
                    working_set.error(e.wrap(working_set, val_span));
                }
            },
            "complete external" => match value {
                Value::Nothing { .. } => {
                    signature.complete = Some(CommandWideCompleter::External);
                }
                _ => {
                    let e = ShellError::GenericError {
                        error: "nu::shell::invalid_completer".into(),
                        msg: "This attribute shouldn't return anything".into(),
                        span: Some(val_span),
                        help: Some("Is `attr complete` shadowed?".into()),
                        inner: vec![],
                    };
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

fn check_alias_name<'a>(working_set: &mut StateWorkingSet, spans: &'a [Span]) -> Option<&'a Span> {
    let command_len = if !spans.is_empty() {
        if working_set.get_span_contents(spans[0]) == b"export" {
            2
        } else {
            1
        }
    } else {
        return None;
    };

    if spans.len() == command_len {
        None
    } else if spans.len() < command_len + 3 {
        if working_set.get_span_contents(spans[command_len]) == b"=" {
            let name = String::from_utf8_lossy(
                working_set.get_span_contents(Span::concat(&spans[..command_len])),
            );
            working_set.error(ParseError::AssignmentMismatch(
                format!("{name} missing name"),
                "missing name".into(),
                spans[command_len],
            ));
            Some(&spans[command_len])
        } else {
            None
        }
    } else if working_set.get_span_contents(spans[command_len + 1]) != b"=" {
        let name = String::from_utf8_lossy(
            working_set.get_span_contents(Span::concat(&spans[..command_len])),
        );
        working_set.error(ParseError::AssignmentMismatch(
            format!("{name} missing sign"),
            "missing equal sign".into(),
            spans[command_len + 1],
        ));
        Some(&spans[command_len + 1])
    } else {
        None
    }
}

pub fn parse_alias(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
    module_name: Option<&[u8]>,
) -> Pipeline {
    let spans = &lite_command.parts;

    let (name_span, split_id) =
        if spans.len() > 1 && working_set.get_span_contents(spans[0]) == b"export" {
            (spans[1], 2)
        } else {
            (spans[0], 1)
        };

    let name = working_set.get_span_contents(name_span);

    if name != b"alias" {
        working_set.error(ParseError::InternalError(
            "Alias statement unparsable".into(),
            Span::concat(spans),
        ));
        return garbage_pipeline(working_set, spans);
    }
    if let Some(redirection) = lite_command.redirection.as_ref() {
        working_set.error(redirecting_builtin_error("alias", redirection));
        return garbage_pipeline(working_set, spans);
    }

    if let Some(span) = check_alias_name(working_set, spans) {
        return Pipeline::from_vec(vec![garbage(working_set, *span)]);
    }

    if let Some(decl_id) = working_set.find_decl(b"alias") {
        let (command_spans, rest_spans) = spans.split_at(split_id);

        let original_starting_error_count = working_set.parse_errors.len();

        let ParsedInternalCall {
            call: alias_call,
            output,
            call_kind,
        } = parse_internal_call(
            working_set,
            Span::concat(command_spans),
            rest_spans,
            decl_id,
        );

        working_set
            .parse_errors
            .truncate(original_starting_error_count);

        let alias_pipeline = Pipeline::from_vec(vec![Expression::new(
            working_set,
            Expr::Call(alias_call.clone()),
            Span::concat(spans),
            output,
        )]);

        if call_kind == CallKind::Help {
            return alias_pipeline;
        }

        let Some(alias_name_expr) = alias_call.positional_nth(0) else {
            working_set.error(ParseError::UnknownState(
                "Missing positional after call check".to_string(),
                Span::concat(spans),
            ));
            return garbage_pipeline(working_set, spans);
        };

        let alias_name = if let Some(name) = alias_name_expr.as_string() {
            if name.contains('#')
                || name.contains('^')
                || name.parse::<bytesize::ByteSize>().is_ok()
                || name.parse::<f64>().is_ok()
            {
                working_set.error(ParseError::AliasNotValid(alias_name_expr.span));
                return garbage_pipeline(working_set, spans);
            } else {
                name
            }
        } else {
            working_set.error(ParseError::AliasNotValid(alias_name_expr.span));
            return garbage_pipeline(working_set, spans);
        };

        if spans.len() >= split_id + 3 {
            if let Some(mod_name) = module_name {
                if alias_name.as_bytes() == mod_name {
                    working_set.error(ParseError::NamedAsModule(
                        "alias".to_string(),
                        alias_name,
                        "main".to_string(),
                        spans[split_id],
                    ));

                    return alias_pipeline;
                }

                if alias_name == "main" {
                    working_set.error(ParseError::ExportMainAliasNotAllowed(spans[split_id]));
                    return alias_pipeline;
                }
            }

            let _equals = working_set.get_span_contents(spans[split_id + 1]);

            let replacement_spans = &spans[(split_id + 2)..];
            let first_bytes = working_set.get_span_contents(replacement_spans[0]);

            if first_bytes != b"if"
                && first_bytes != b"match"
                && is_math_expression_like(working_set, replacement_spans[0])
            {
                // TODO: Maybe we need to implement a Display trait for Expression?
                let starting_error_count = working_set.parse_errors.len();
                let expr = parse_expression(working_set, replacement_spans);
                working_set.parse_errors.truncate(starting_error_count);

                let msg = format!("{:?}", expr.expr);
                let msg_parts: Vec<&str> = msg.split('(').collect();

                working_set.error(ParseError::CantAliasExpression(
                    msg_parts[0].to_string(),
                    replacement_spans[0],
                ));
                return alias_pipeline;
            }

            let starting_error_count = working_set.parse_errors.len();
            working_set.search_predecls = false;

            let expr = parse_call(working_set, replacement_spans, replacement_spans[0]);

            working_set.search_predecls = true;

            if starting_error_count != working_set.parse_errors.len()
                && let Some(e) = working_set.parse_errors.get(starting_error_count)
            {
                if let ParseError::MissingPositional(..) = e {
                    working_set
                        .parse_errors
                        .truncate(original_starting_error_count);
                    // ignore missing required positional
                } else {
                    return garbage_pipeline(working_set, replacement_spans);
                }
            }

            let (command, wrapped_call) = match expr {
                Expression {
                    expr: Expr::Call(ref rhs_call),
                    ..
                } => {
                    let cmd = working_set.get_decl(rhs_call.decl_id);

                    if cmd.is_keyword()
                        && !ALIASABLE_PARSER_KEYWORDS.contains(&cmd.name().as_bytes())
                    {
                        working_set.error(ParseError::CantAliasKeyword(
                            ALIASABLE_PARSER_KEYWORDS
                                .iter()
                                .map(|bytes| String::from_utf8_lossy(bytes).to_string())
                                .collect::<Vec<String>>()
                                .join(", "),
                            rhs_call.head,
                        ));
                        return alias_pipeline;
                    }

                    (Some(cmd.clone_box()), expr)
                }
                Expression {
                    expr: Expr::ExternalCall(..),
                    ..
                } => (None, expr),
                _ => {
                    working_set.error(ParseError::InternalError(
                        "Parsed call not a call".into(),
                        expr.span,
                    ));
                    return alias_pipeline;
                }
            };

            // Tries to build a useful description string
            let (description, extra_description) = match lite_command.comments.is_empty() {
                // First from comments, if any are present
                false => working_set.build_desc(&lite_command.comments),
                // Then from the command itself
                true => match alias_call.arguments.get(1) {
                    Some(Argument::Positional(Expression {
                        expr: Expr::Keyword(kw),
                        ..
                    })) => {
                        let aliased = working_set.get_span_contents(kw.expr.span);
                        (
                            format!("Alias for `{}`", String::from_utf8_lossy(aliased)),
                            String::new(),
                        )
                    }
                    // Then with a default.
                    _ => ("User declared alias".into(), String::new()),
                },
            };

            let decl = Alias {
                name: alias_name,
                command,
                wrapped_call,
                description,
                extra_description,
            };

            working_set.add_decl(Box::new(decl));
        }

        // special case for `alias foo=bar`
        if spans.len() == 2 && working_set.get_span_contents(spans[1]).contains(&b'=') {
            let arg = String::from_utf8_lossy(working_set.get_span_contents(spans[1]));

            // split at '='.  Note that the output must never be None, the
            // `unwrap` is just to avoid the possibility of panic, if the
            // invariant is broken.
            let (name, initial_value) = arg.split_once('=').unwrap_or((&arg, ""));

            let name = if name.is_empty() { "{name}" } else { name };
            let initial_value = if initial_value.is_empty() {
                "{initial_value}"
            } else {
                initial_value
            };

            working_set.error(ParseError::IncorrectValue(
                "alias argument".into(),
                spans[1],
                format!("Make sure to put spaces around '=': alias {name} = {initial_value}"),
            ))
        } else if spans.len() < 4 {
            working_set.error(ParseError::IncorrectValue(
                "Incomplete alias".into(),
                Span::concat(&spans[..split_id]),
                "incomplete alias".into(),
            ));
        }

        return alias_pipeline;
    }

    working_set.error(ParseError::InternalError(
        "Alias statement unparsable".into(),
        Span::concat(spans),
    ));

    garbage_pipeline(working_set, spans)
}

// Return false if command `export xxx` not found
// TODO: Rather than this, handle `export xxx` correctly in `parse_xxx`
fn warp_export_call(
    working_set: &mut StateWorkingSet,
    pipeline: &mut Pipeline,
    full_name: &str,
    spans: &[Span],
) -> bool {
    let Some(export_decl_id) = working_set.find_decl(full_name.as_bytes()) else {
        let error_span = spans.first().cloned().unwrap_or(Span::unknown());
        working_set.error(ParseError::InternalError(
            format!("missing '{full_name}' command"),
            error_span,
        ));
        return false;
    };
    match pipeline.elements.first_mut().map(|e| {
        e.expr.span = Span::concat(spans);
        &mut e.expr.expr
    }) {
        Some(Expr::Call(def_call)) => {
            def_call.head = Span::concat(&spans[0..=1]);
            def_call.decl_id = export_decl_id;
            return true;
        }
        Some(Expr::AttributeBlock(ab)) => {
            if let Expr::Call(def_call) = &mut ab.item.expr {
                def_call.head = Span::concat(&spans[0..=1]);
                def_call.decl_id = export_decl_id;
                return true;
            }
        }
        _ => {}
    };
    working_set.error(ParseError::InternalError(
        "unexpected output from parsing a definition".into(),
        Span::concat(&spans[1..]),
    ));
    true
}

// This one will trigger if `export` appears during eval, e.g., in a script
pub fn parse_export_in_block(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
) -> Pipeline {
    let parts = lite_command.command_parts();
    let full_name = if parts.len() > 1 {
        let sub = working_set.get_span_contents(parts[1]);
        match sub {
            b"alias" => "export alias",
            b"def" => "export def",
            b"extern" => "export extern",
            b"use" => "export use",
            b"module" => "export module",
            b"const" => "export const",
            _ => "export",
        }
    } else {
        "export"
    };

    if let Some(redirection) = lite_command.redirection.as_ref() {
        working_set.error(redirecting_builtin_error(full_name, redirection));
        return garbage_pipeline(working_set, &lite_command.parts);
    }

    let mut pipeline = match full_name {
        // `parse_def` and `parse_extern` work both with and without attributes
        "export def" => parse_def(working_set, lite_command, None).0,
        "export extern" => parse_extern(working_set, lite_command, None),
        // Other definitions can't have attributes, so we handle attributes here with parse_attribute_block
        _ if lite_command.has_attributes() => parse_attribute_block(working_set, lite_command),
        "export alias" => parse_alias(working_set, lite_command, None),
        "export const" => parse_const(working_set, &lite_command.parts[1..]).0,
        "export use" => parse_use(working_set, lite_command, None).0,
        "export module" => parse_module(working_set, lite_command, None).0,
        _ => {
            if let Some(decl_id) = working_set.find_decl(full_name.as_bytes()) {
                let starting_error_count = working_set.parse_errors.len();
                let ParsedInternalCall {
                    call,
                    output,
                    call_kind,
                } = parse_internal_call(working_set, parts[0], &parts[1..], decl_id);

                if call_kind != CallKind::Valid {
                    return Pipeline::from_vec(vec![Expression::new(
                        working_set,
                        Expr::Call(call),
                        Span::concat(&lite_command.parts),
                        output,
                    )]);
                }
                // don't need errors generated by parse_internal_call
                working_set.parse_errors.truncate(starting_error_count);
                working_set.error(ParseError::UnexpectedKeyword(
                    full_name.into(),
                    lite_command.parts[0],
                ));
            } else {
                working_set.error(ParseError::UnknownState(
                    format!("internal error: '{full_name}' declaration not found",),
                    Span::concat(&lite_command.parts),
                ));
            };
            garbage_pipeline(working_set, &lite_command.parts)
        }
    };

    // HACK: This is for different messages of e.g. `export def --help` and `def --help`,
    warp_export_call(working_set, &mut pipeline, full_name, &lite_command.parts);
    pipeline
}

// This one will trigger only in a module
pub fn parse_export_in_module(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
    module_name: &[u8],
    parent_module: &mut Module,
) -> (Pipeline, Vec<Exportable>) {
    let spans = lite_command.command_parts();

    let export_span = if let Some(sp) = spans.first() {
        if working_set.get_span_contents(*sp) != b"export" {
            working_set.error(ParseError::UnknownState(
                "expected export statement".into(),
                Span::concat(spans),
            ));
            return (garbage_pipeline(working_set, spans), vec![]);
        }

        *sp
    } else {
        working_set.error(ParseError::UnknownState(
            "got empty input for parsing export statement".into(),
            Span::concat(spans),
        ));
        return (garbage_pipeline(working_set, spans), vec![]);
    };

    let (pipeline, exportables) = if let Some(kw_span) = spans.get(1) {
        let kw_name = working_set.get_span_contents(*kw_span);
        match kw_name {
            // `parse_def` and `parse_extern` work both with and without attributes
            b"def" => {
                let (mut pipeline, cmd_result) =
                    parse_def(working_set, lite_command, Some(module_name));

                let mut result = vec![];

                if let Some((decl_name, decl_id)) = cmd_result {
                    result.push(Exportable::Decl {
                        name: decl_name.to_vec(),
                        id: decl_id,
                    });
                }

                // Trying to warp the 'def' call into the 'export def' in a very clumsy way
                if !warp_export_call(working_set, &mut pipeline, "export def", spans) {
                    return (garbage_pipeline(working_set, spans), vec![]);
                }

                (pipeline, result)
            }
            b"extern" => {
                let mut pipeline = parse_extern(working_set, lite_command, Some(module_name));

                // Trying to warp the 'extern' call into the 'export extern' in a very clumsy way
                if !warp_export_call(working_set, &mut pipeline, "export extern", spans) {
                    return (garbage_pipeline(working_set, spans), vec![]);
                }

                let mut result = vec![];

                let decl_name = match spans.get(2) {
                    Some(span) => working_set.get_span_contents(*span),
                    None => &[],
                };
                let decl_name = trim_quotes(decl_name);

                if let Some(decl_id) = working_set.find_decl(decl_name) {
                    result.push(Exportable::Decl {
                        name: decl_name.to_vec(),
                        id: decl_id,
                    });
                } else {
                    working_set.error(ParseError::InternalError(
                        "failed to find added declaration".into(),
                        Span::concat(&spans[1..]),
                    ));
                }

                (pipeline, result)
            }
            // Other definitions can't have attributes, so we handle attributes here with parse_attribute_block
            _ if lite_command.has_attributes() => {
                (parse_attribute_block(working_set, lite_command), vec![])
            }
            b"alias" => {
                let lite_command = LiteCommand {
                    comments: lite_command.comments.clone(),
                    parts: spans[1..].to_vec(),
                    pipe: lite_command.pipe,
                    redirection: lite_command.redirection.clone(),
                    attribute_idx: vec![],
                };
                let mut pipeline = parse_alias(working_set, &lite_command, Some(module_name));

                // Trying to warp the 'alias' call into the 'export alias' in a very clumsy way
                if !warp_export_call(working_set, &mut pipeline, "export alias", spans) {
                    return (garbage_pipeline(working_set, spans), vec![]);
                }

                let mut result = vec![];

                let alias_name = match spans.get(2) {
                    Some(span) => working_set.get_span_contents(*span),
                    None => &[],
                };
                let alias_name = trim_quotes(alias_name);

                if let Some(alias_id) = working_set.find_decl(alias_name) {
                    result.push(Exportable::Decl {
                        name: alias_name.to_vec(),
                        id: alias_id,
                    });
                } else {
                    working_set.error(ParseError::InternalError(
                        "failed to find added alias".into(),
                        Span::concat(&spans[1..]),
                    ));
                }

                (pipeline, result)
            }
            b"use" => {
                let lite_command = LiteCommand {
                    comments: lite_command.comments.clone(),
                    parts: spans[1..].to_vec(),
                    pipe: lite_command.pipe,
                    redirection: lite_command.redirection.clone(),
                    attribute_idx: vec![],
                };
                let (mut pipeline, exportables) =
                    parse_use(working_set, &lite_command, Some(parent_module));

                // Trying to warp the 'use' call into the 'export use' in a very clumsy way
                if !warp_export_call(working_set, &mut pipeline, "export use", spans) {
                    return (garbage_pipeline(working_set, spans), vec![]);
                }

                (pipeline, exportables)
            }
            b"module" => {
                let (mut pipeline, maybe_module_id) =
                    parse_module(working_set, lite_command, Some(module_name));

                // Trying to warp the 'module' call into the 'export module' in a very clumsy way
                if !warp_export_call(working_set, &mut pipeline, "export module", spans) {
                    return (garbage_pipeline(working_set, spans), vec![]);
                }

                let mut result = vec![];

                if let Some(module_name_span) = spans.get(2) {
                    let module_name = working_set.get_span_contents(*module_name_span);
                    let module_name = trim_quotes(module_name);

                    if let Some(module_id) = maybe_module_id {
                        result.push(Exportable::Module {
                            name: working_set.get_module(module_id).name(),
                            id: module_id,
                        });
                    } else {
                        working_set.error(ParseError::InternalError(
                            format!(
                                "failed to find added module '{}'",
                                String::from_utf8_lossy(module_name)
                            ),
                            Span::concat(&spans[1..]),
                        ));
                    }
                }

                (pipeline, result)
            }
            b"const" => {
                let (mut pipeline, var_name_span) = parse_const(working_set, &spans[1..]);

                // Trying to warp the 'const' call into the 'export const' in a very clumsy way
                if !warp_export_call(working_set, &mut pipeline, "export const", spans) {
                    return (garbage_pipeline(working_set, spans), vec![]);
                }

                let mut result = vec![];

                if let Some(var_name_span) = var_name_span {
                    let var_name = working_set.get_span_contents(var_name_span);
                    let var_name = trim_quotes(var_name);

                    if let Some(var_id) = working_set.find_variable(var_name) {
                        if let Err(err) = working_set.get_constant(var_id) {
                            working_set.error(err);
                        } else {
                            result.push(Exportable::VarDecl {
                                name: var_name.to_vec(),
                                id: var_id,
                            });
                        }
                    } else {
                        working_set.error(ParseError::InternalError(
                            "failed to find added variable".into(),
                            Span::concat(&spans[1..]),
                        ));
                    }
                }

                (pipeline, result)
            }
            _ => {
                working_set.error(ParseError::Expected(
                    "def, alias, use, module, const or extern keyword",
                    spans[1],
                ));

                (garbage_pipeline(working_set, spans), vec![])
            }
        }
    } else {
        working_set.error(ParseError::MissingPositional(
            "def, alias, use, module, const or extern keyword".to_string(),
            Span::new(export_span.end, export_span.end),
            "def, alias, use, module, const or extern keyword".to_string(),
        ));

        (garbage_pipeline(working_set, spans), vec![])
    };

    (pipeline, exportables)
}

pub fn parse_export_env(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
) -> (Pipeline, Option<BlockId>) {
    if !spans.is_empty() && working_set.get_span_contents(spans[0]) != b"export-env" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for 'export-env' command".into(),
            Span::concat(spans),
        ));
        return (garbage_pipeline(working_set, spans), None);
    }

    if spans.len() < 2 {
        working_set.error(ParseError::MissingPositional(
            "block".into(),
            Span::concat(spans),
            "export-env <block>".into(),
        ));
        return (garbage_pipeline(working_set, spans), None);
    }

    let call = match working_set.find_decl(b"export-env") {
        Some(decl_id) => {
            let ParsedInternalCall {
                call,
                output,
                call_kind,
            } = parse_internal_call(working_set, spans[0], &[spans[1]], decl_id);

            if call_kind != CallKind::Valid {
                return (
                    Pipeline::from_vec(vec![Expression::new(
                        working_set,
                        Expr::Call(call),
                        Span::concat(spans),
                        output,
                    )]),
                    None,
                );
            }

            call
        }
        None => {
            working_set.error(ParseError::UnknownState(
                "internal error: 'export-env' declaration not found".into(),
                Span::concat(spans),
            ));
            return (garbage_pipeline(working_set, spans), None);
        }
    };

    let block_id = if let Some(block) = call.positional_nth(0) {
        if let Some(block_id) = block.as_block() {
            block_id
        } else {
            working_set.error(ParseError::UnknownState(
                "internal error: 'export-env' block is not a block".into(),
                block.span,
            ));
            return (garbage_pipeline(working_set, spans), None);
        }
    } else {
        working_set.error(ParseError::UnknownState(
            "internal error: 'export-env' block is missing".into(),
            Span::concat(spans),
        ));
        return (garbage_pipeline(working_set, spans), None);
    };

    let pipeline = Pipeline::from_vec(vec![Expression::new(
        working_set,
        Expr::Call(call),
        Span::concat(spans),
        Type::Any,
    )]);

    // Since modules are parser constructs, `export-env` blocks don't have anything to drive their
    // compilation when they are inside modules
    //
    // When they appear not inside module definitions but inside runnable code (script, `def`
    // block, etc), their body is used more or less like a closure, which are also compiled eagerly
    //
    // This handles both of these cases
    compile_block_with_id(working_set, block_id);

    (pipeline, Some(block_id))
}

fn collect_first_comments(tokens: &[Token]) -> Vec<Span> {
    let mut comments = vec![];

    let mut tokens_iter = tokens.iter().peekable();
    while let Some(token) = tokens_iter.next() {
        match token.contents {
            TokenContents::Comment => {
                comments.push(token.span);
            }
            TokenContents::Eol => {
                if let Some(Token {
                    contents: TokenContents::Eol,
                    ..
                }) = tokens_iter.peek()
                    && !comments.is_empty()
                {
                    break;
                }
            }
            _ => {
                comments.clear();
                break;
            }
        }
    }

    comments
}

pub fn parse_module_block(
    working_set: &mut StateWorkingSet,
    span: Span,
    module_name: &[u8],
) -> (Block, Module, Vec<Span>) {
    working_set.enter_scope();

    let source = working_set.get_span_contents(span);

    let (output, err) = lex(source, span.start, &[], &[], false);
    if let Some(err) = err {
        working_set.error(err)
    }

    let module_comments = collect_first_comments(&output);

    let (output, err) = lite_parse(&output, working_set);
    if let Some(err) = err {
        working_set.error(err)
    }

    for pipeline in &output.block {
        if pipeline.commands.len() == 1 {
            parse_def_predecl(working_set, pipeline.commands[0].command_parts());
        }
    }

    let mut module = Module::from_span(module_name.to_vec(), span);

    let mut block = Block::new_with_capacity(output.block.len());
    block.span = Some(span);

    for pipeline in output.block.iter() {
        if pipeline.commands.len() == 1 {
            let command = &pipeline.commands[0];

            let name = command
                .command_parts()
                .first()
                .map(|s| working_set.get_span_contents(*s))
                .unwrap_or(b"");

            match name {
                // `parse_def` and `parse_extern` work both with and without attributes
                b"def" => {
                    block.pipelines.push(
                        parse_def(
                            working_set,
                            command,
                            None, // using commands named as the module locally is OK
                        )
                        .0,
                    )
                }
                b"extern" => block
                    .pipelines
                    .push(parse_extern(working_set, command, None)),
                // `parse_export_in_module` also handles attributes by itself
                b"export" => {
                    let (pipe, exportables) =
                        parse_export_in_module(working_set, command, module_name, &mut module);

                    for exportable in exportables {
                        match exportable {
                            Exportable::Decl { name, id } => {
                                if &name == b"main" {
                                    if module.main.is_some() {
                                        let err_span = if !pipe.elements.is_empty() {
                                            if let Expr::Call(call) = &pipe.elements[0].expr.expr {
                                                call.head
                                            } else {
                                                pipe.elements[0].expr.span
                                            }
                                        } else {
                                            span
                                        };
                                        working_set.error(ParseError::ModuleDoubleMain(
                                            String::from_utf8_lossy(module_name).to_string(),
                                            err_span,
                                        ));
                                    } else {
                                        module.main = Some(id);
                                    }
                                } else {
                                    module.add_decl(name, id);
                                }
                            }
                            Exportable::Module { name, id } => {
                                if &name == b"mod" {
                                    let (submodule_main, submodule_decls, submodule_submodules) = {
                                        let submodule = working_set.get_module(id);
                                        (submodule.main, submodule.decls(), submodule.submodules())
                                    };

                                    // Add submodule's decls to the parent module
                                    for (decl_name, decl_id) in submodule_decls {
                                        module.add_decl(decl_name, decl_id);
                                    }

                                    // Add submodule's main command to the parent module
                                    if let Some(main_decl_id) = submodule_main {
                                        if module.main.is_some() {
                                            let err_span = if !pipe.elements.is_empty() {
                                                if let Expr::Call(call) =
                                                    &pipe.elements[0].expr.expr
                                                {
                                                    call.head
                                                } else {
                                                    pipe.elements[0].expr.span
                                                }
                                            } else {
                                                span
                                            };
                                            working_set.error(ParseError::ModuleDoubleMain(
                                                String::from_utf8_lossy(module_name).to_string(),
                                                err_span,
                                            ));
                                        } else {
                                            module.main = Some(main_decl_id);
                                        }
                                    }

                                    // Add submodule's submodules to the parent module
                                    for (submodule_name, submodule_id) in submodule_submodules {
                                        module.add_submodule(submodule_name, submodule_id);
                                    }
                                } else {
                                    module.add_submodule(name, id);
                                }
                            }
                            Exportable::VarDecl { name, id } => {
                                module.add_variable(name, id);
                            }
                        }
                    }

                    block.pipelines.push(pipe)
                }
                // Other definitions can't have attributes, so we handle attributes here with parse_attribute_block
                _ if command.has_attributes() => block
                    .pipelines
                    .push(parse_attribute_block(working_set, command)),
                b"const" => block
                    .pipelines
                    .push(parse_const(working_set, &command.parts).0),
                b"alias" => {
                    block.pipelines.push(parse_alias(
                        working_set,
                        command,
                        None, // using aliases named as the module locally is OK
                    ))
                }
                b"use" => {
                    let (pipeline, _) = parse_use(working_set, command, Some(&mut module));

                    block.pipelines.push(pipeline)
                }
                b"module" => {
                    let (pipeline, _) = parse_module(
                        working_set,
                        command,
                        None, // using modules named as the module locally is OK
                    );

                    block.pipelines.push(pipeline)
                }
                b"export-env" => {
                    let (pipe, maybe_env_block) = parse_export_env(working_set, &command.parts);

                    if let Some(block_id) = maybe_env_block {
                        module.add_env_block(block_id);
                    }

                    block.pipelines.push(pipe)
                }
                _ => {
                    working_set.error(ParseError::ExpectedKeyword(
                        "def, const, extern, alias, use, module, export or export-env keyword"
                            .into(),
                        command.parts[0],
                    ));

                    block
                        .pipelines
                        .push(garbage_pipeline(working_set, &command.parts))
                }
            }
        } else {
            working_set.error(ParseError::Expected("not a pipeline", span));
            block.pipelines.push(garbage_pipeline(working_set, &[span]))
        }
    }

    working_set.exit_scope();

    (block, module, module_comments)
}

fn module_needs_reloading(working_set: &StateWorkingSet, module_id: ModuleId) -> bool {
    let module = working_set.get_module(module_id);

    fn submodule_need_reloading(working_set: &StateWorkingSet, submodule_id: ModuleId) -> bool {
        let submodule = working_set.get_module(submodule_id);
        let submodule_changed = if let Some((file_path, file_id)) = &submodule.file {
            let existing_contents = working_set.get_contents_of_file(*file_id);
            let file_contents = file_path.read(working_set);

            if let (Some(existing), Some(new)) = (existing_contents, file_contents) {
                existing != new
            } else {
                false
            }
        } else {
            false
        };

        if submodule_changed {
            true
        } else {
            module_needs_reloading(working_set, submodule_id)
        }
    }

    let export_submodule_changed = module
        .submodules
        .iter()
        .any(|(_, submodule_id)| submodule_need_reloading(working_set, *submodule_id));

    if export_submodule_changed {
        return true;
    }

    module
        .imported_modules
        .iter()
        .any(|submodule_id| submodule_need_reloading(working_set, *submodule_id))
}

/// Parse a module from a file.
///
/// The module name is inferred from the stem of the file, unless specified in `name_override`.
fn parse_module_file(
    working_set: &mut StateWorkingSet,
    path: ParserPath,
    path_span: Span,
    name_override: Option<String>,
) -> Option<ModuleId> {
    // Infer the module name from the stem of the file, unless overridden.
    let module_name = if let Some(name) = name_override {
        name
    } else if let Some(stem) = path.file_stem() {
        stem.to_string_lossy().to_string()
    } else {
        working_set.error(ParseError::ModuleNotFound(
            path_span,
            path.path().to_string_lossy().to_string(),
        ));
        return None;
    };

    // Read the content of the module.
    let contents = if let Some(contents) = path.read(working_set) {
        contents
    } else {
        working_set.error(ParseError::ModuleNotFound(
            path_span,
            path.path().to_string_lossy().to_string(),
        ));
        return None;
    };

    let file_id = working_set.add_file(path.path().to_string_lossy().to_string(), &contents);
    let new_span = working_set.get_span_for_file(file_id);

    // Check if we've parsed the module before.
    if let Some(module_id) = working_set.find_module_by_span(new_span)
        && !module_needs_reloading(working_set, module_id)
    {
        return Some(module_id);
    }

    // Add the file to the stack of files being processed.
    if let Err(e) = working_set.files.push(path.clone().path_buf(), path_span) {
        working_set.error(e);
        return None;
    }

    // Parse the module
    let (block, mut module, module_comments) =
        parse_module_block(working_set, new_span, module_name.as_bytes());

    // Remove the file from the stack of files being processed.
    working_set.files.pop();

    let _ = working_set.add_block(Arc::new(block));
    module.file = Some((path, file_id));
    let module_id = working_set.add_module(&module_name, module, module_comments);

    Some(module_id)
}

pub fn parse_module_file_or_dir(
    working_set: &mut StateWorkingSet,
    path: &[u8],
    path_span: Span,
    name_override: Option<String>,
) -> Option<ModuleId> {
    let (module_path_str, err) = unescape_unquote_string(path, path_span);
    if let Some(err) = err {
        working_set.error(err);
        return None;
    }

    #[allow(deprecated)]
    let cwd = working_set.get_cwd();

    let module_path =
        if let Some(path) = find_in_dirs(&module_path_str, working_set, &cwd, Some(LIB_DIRS_VAR)) {
            path
        } else {
            working_set.error(ParseError::ModuleNotFound(path_span, module_path_str));
            return None;
        };

    if module_path.is_dir() {
        if module_path.read_dir().is_none() {
            working_set.error(ParseError::ModuleNotFound(
                path_span,
                module_path.path().to_string_lossy().to_string(),
            ));
            return None;
        };

        let module_name = if let Some(stem) = module_path.file_stem() {
            stem.to_string_lossy().to_string()
        } else {
            working_set.error(ParseError::ModuleNotFound(
                path_span,
                module_path.path().to_string_lossy().to_string(),
            ));
            return None;
        };

        let mod_nu_path = module_path.clone().join("mod.nu");

        if !(mod_nu_path.exists() && mod_nu_path.is_file()) {
            working_set.error(ParseError::ModuleMissingModNuFile(
                module_path.path().to_string_lossy().to_string(),
                path_span,
            ));
            return None;
        }

        if let Some(module_id) = parse_module_file(
            working_set,
            mod_nu_path,
            path_span,
            name_override.or(Some(module_name)),
        ) {
            let module = working_set.get_module(module_id).clone();

            let module_name = String::from_utf8_lossy(&module.name).to_string();

            let module_comments = if let Some(comments) = working_set.get_module_comments(module_id)
            {
                comments.to_vec()
            } else {
                vec![]
            };

            let new_module_id = working_set.add_module(&module_name, module, module_comments);

            Some(new_module_id)
        } else {
            None
        }
    } else if module_path.is_file() {
        parse_module_file(working_set, module_path, path_span, name_override)
    } else {
        working_set.error(ParseError::ModuleNotFound(
            path_span,
            module_path.path().to_string_lossy().to_string(),
        ));
        None
    }
}

pub fn parse_module(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
    module_name: Option<&[u8]>,
) -> (Pipeline, Option<ModuleId>) {
    // TODO: Currently, module is closing over its parent scope (i.e., defs in the parent scope are
    // visible and usable in this module's scope). We want to disable that for files.

    let spans = &lite_command.parts;

    if let Some(redirection) = lite_command.redirection.as_ref() {
        working_set.error(redirecting_builtin_error("module", redirection));
        return (garbage_pipeline(working_set, spans), None);
    }

    let mut module_comments = lite_command.comments.clone();

    let split_id = if spans.len() > 1 && working_set.get_span_contents(spans[0]) == b"export" {
        2
    } else {
        1
    };

    let (call, call_span) = match working_set.find_decl(b"module") {
        Some(decl_id) => {
            let (command_spans, rest_spans) = spans.split_at(split_id);

            let ParsedInternalCall {
                call,
                output,
                call_kind,
            } = parse_internal_call(
                working_set,
                Span::concat(command_spans),
                rest_spans,
                decl_id,
            );

            let call_span = Span::concat(spans);
            if call_kind != CallKind::Valid {
                return (
                    Pipeline::from_vec(vec![Expression::new(
                        working_set,
                        Expr::Call(call),
                        call_span,
                        output,
                    )]),
                    None,
                );
            }

            (call, call_span)
        }
        None => {
            working_set.error(ParseError::UnknownState(
                "internal error: 'module' or 'export module' declaration not found".into(),
                Span::concat(spans),
            ));
            return (garbage_pipeline(working_set, spans), None);
        }
    };

    let (module_name_or_path, module_name_or_path_span, module_name_or_path_expr) =
        if let Some(name) = call.positional_nth(0) {
            if let Some(s) = name.as_string() {
                if let Some(mod_name) = module_name
                    && s.as_bytes() == mod_name
                {
                    working_set.error(ParseError::NamedAsModule(
                        "module".to_string(),
                        s,
                        "mod".to_string(),
                        name.span,
                    ));
                    return (
                        Pipeline::from_vec(vec![Expression::new(
                            working_set,
                            Expr::Call(call),
                            call_span,
                            Type::Any,
                        )]),
                        None,
                    );
                }
                (s, name.span, name.clone())
            } else {
                working_set.error(ParseError::UnknownState(
                    "internal error: name not a string".into(),
                    Span::concat(spans),
                ));
                return (garbage_pipeline(working_set, spans), None);
            }
        } else {
            working_set.error(ParseError::UnknownState(
                "internal error: missing positional".into(),
                Span::concat(spans),
            ));
            return (garbage_pipeline(working_set, spans), None);
        };

    let pipeline = Pipeline::from_vec(vec![Expression::new(
        working_set,
        Expr::Call(call),
        call_span,
        Type::Any,
    )]);

    if spans.len() == split_id + 1 {
        if let Some(module_id) = parse_module_file_or_dir(
            working_set,
            module_name_or_path.as_bytes(),
            module_name_or_path_span,
            None,
        ) {
            return (pipeline, Some(module_id));
        } else {
            working_set.error(ParseError::ModuleNotFound(
                module_name_or_path_span,
                module_name_or_path,
            ));
            return (pipeline, None);
        }
    }

    if spans.len() < split_id + 2 {
        working_set.error(ParseError::UnknownState(
            "Expected structure: module <name> or module <name> <block>".into(),
            Span::concat(spans),
        ));

        return (garbage_pipeline(working_set, spans), None);
    }

    let module_name = module_name_or_path;

    let block_expr_span = spans[split_id + 1];
    let block_bytes = working_set.get_span_contents(block_expr_span);
    let mut start = block_expr_span.start;
    let mut end = block_expr_span.end;

    if block_bytes.starts_with(b"{") {
        start += 1;
    } else {
        working_set.error(ParseError::Expected("block", block_expr_span));
        return (garbage_pipeline(working_set, spans), None);
    }

    if block_bytes.ends_with(b"}") {
        end -= 1;
    } else {
        working_set.error(ParseError::Unclosed("}".into(), Span::new(end, end)));
    }

    let block_content_span = Span::new(start, end);

    let (block, module, inner_comments) =
        parse_module_block(working_set, block_content_span, module_name.as_bytes());

    let block_id = working_set.add_block(Arc::new(block));

    module_comments.extend(inner_comments);
    let module_id = working_set.add_module(&module_name, module, module_comments);

    let block_expr = Expression::new(
        working_set,
        Expr::Block(block_id),
        block_expr_span,
        Type::Block,
    );

    let module_decl_id = working_set
        .find_decl(b"module")
        .expect("internal error: missing module command");

    let call = Box::new(Call {
        head: Span::concat(&spans[..split_id]),
        decl_id: module_decl_id,
        arguments: vec![
            Argument::Positional(module_name_or_path_expr),
            Argument::Positional(block_expr),
        ],
        parser_info: HashMap::new(),
    });

    (
        Pipeline::from_vec(vec![Expression::new(
            working_set,
            Expr::Call(call),
            Span::concat(spans),
            Type::Any,
        )]),
        Some(module_id),
    )
}

pub fn parse_use(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
    parent_module: Option<&mut Module>,
) -> (Pipeline, Vec<Exportable>) {
    let spans = &lite_command.parts;

    let (name_span, split_id) =
        if spans.len() > 1 && working_set.get_span_contents(spans[0]) == b"export" {
            (spans[1], 2)
        } else {
            (spans[0], 1)
        };

    let use_call = working_set.get_span_contents(name_span).to_vec();
    if use_call != b"use" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for 'use' command".into(),
            Span::concat(spans),
        ));
        return (garbage_pipeline(working_set, spans), vec![]);
    }

    if working_set.get_span_contents(name_span) != b"use" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for 'use' command".into(),
            Span::concat(spans),
        ));
        return (garbage_pipeline(working_set, spans), vec![]);
    }

    if let Some(redirection) = lite_command.redirection.as_ref() {
        working_set.error(redirecting_builtin_error("use", redirection));
        return (garbage_pipeline(working_set, spans), vec![]);
    }

    let (call, call_span, args_spans) = match working_set.find_decl(b"use") {
        Some(decl_id) => {
            let (command_spans, rest_spans) = spans.split_at(split_id);

            let ParsedInternalCall {
                call,
                output,
                call_kind,
            } = parse_internal_call(
                working_set,
                Span::concat(command_spans),
                rest_spans,
                decl_id,
            );

            let call_span = Span::concat(spans);
            if call_kind != CallKind::Valid {
                return (
                    Pipeline::from_vec(vec![Expression::new(
                        working_set,
                        Expr::Call(call),
                        call_span,
                        output,
                    )]),
                    vec![],
                );
            }

            (call, call_span, rest_spans)
        }
        None => {
            working_set.error(ParseError::UnknownState(
                "internal error: 'use' declaration not found".into(),
                Span::concat(spans),
            ));
            return (garbage_pipeline(working_set, spans), vec![]);
        }
    };

    let import_pattern_expr = parse_import_pattern(working_set, args_spans);

    let import_pattern = match &import_pattern_expr {
        Expression {
            expr: Expr::Nothing,
            ..
        } => {
            let mut call = call;
            call.set_parser_info(
                "noop".to_string(),
                Expression::new_unknown(Expr::Nothing, Span::unknown(), Type::Nothing),
            );
            return (
                Pipeline::from_vec(vec![Expression::new(
                    working_set,
                    Expr::Call(call),
                    Span::concat(spans),
                    Type::Any,
                )]),
                vec![],
            );
        }
        Expression {
            expr: Expr::ImportPattern(import_pattern),
            ..
        } => import_pattern.clone(),
        _ => {
            working_set.error(ParseError::UnknownState(
                "internal error: Import pattern positional is not import pattern".into(),
                import_pattern_expr.span,
            ));
            return (garbage_pipeline(working_set, spans), vec![]);
        }
    };

    let (mut import_pattern, module, module_id) = if let Some(module_id) = import_pattern.head.id {
        let module = working_set.get_module(module_id).clone();
        (
            ImportPattern {
                head: ImportPatternHead {
                    name: module.name.clone(),
                    id: Some(module_id),
                    span: import_pattern.head.span,
                },
                members: import_pattern.members,
                hidden: HashSet::new(),
                constants: vec![],
            },
            module,
            module_id,
        )
    } else if let Some(module_id) = parse_module_file_or_dir(
        working_set,
        &import_pattern.head.name,
        import_pattern.head.span,
        None,
    ) {
        let module = working_set.get_module(module_id).clone();
        (
            ImportPattern {
                head: ImportPatternHead {
                    name: module.name.clone(),
                    id: Some(module_id),
                    span: import_pattern.head.span,
                },
                members: import_pattern.members,
                hidden: HashSet::new(),
                constants: vec![],
            },
            module,
            module_id,
        )
    } else {
        working_set.error(ParseError::ModuleNotFound(
            import_pattern.head.span,
            String::from_utf8_lossy(&import_pattern.head.name).to_string(),
        ));
        return (
            Pipeline::from_vec(vec![Expression::new(
                working_set,
                Expr::Call(call),
                call_span,
                Type::Any,
            )]),
            vec![],
        );
    };

    let mut imported_modules = vec![];
    let (definitions, errors) = module.resolve_import_pattern(
        working_set,
        module_id,
        &import_pattern.members,
        None,
        name_span,
        &mut imported_modules,
    );

    working_set.parse_errors.extend(errors);

    let mut constants = vec![];

    for (name, const_vid) in definitions.constants {
        constants.push((name, const_vid));
    }

    for (name, const_val) in definitions.constant_values {
        let const_var_id =
            working_set.add_variable(name.clone(), name_span, const_val.get_type(), false);
        working_set.set_variable_const_val(const_var_id, const_val);
        constants.push((name, const_var_id));
    }

    let exportables = definitions
        .decls
        .iter()
        .map(|(name, decl_id)| Exportable::Decl {
            name: name.clone(),
            id: *decl_id,
        })
        .chain(
            definitions
                .modules
                .iter()
                .map(|(name, module_id)| Exportable::Module {
                    name: name.clone(),
                    id: *module_id,
                }),
        )
        .chain(
            constants
                .iter()
                .map(|(name, variable_id)| Exportable::VarDecl {
                    name: name.clone(),
                    id: *variable_id,
                }),
        )
        .collect();

    import_pattern.constants = constants.iter().map(|(_, id)| *id).collect();

    if let Some(m) = parent_module {
        m.track_imported_modules(&imported_modules)
    }
    // Extend the current scope with the module's exportables
    working_set.use_decls(definitions.decls);
    working_set.use_modules(definitions.modules);
    working_set.use_variables(constants);

    // Create a new Use command call to pass the import pattern as parser info
    let import_pattern_expr = Expression::new(
        working_set,
        Expr::ImportPattern(Box::new(import_pattern)),
        Span::concat(args_spans),
        Type::Any,
    );

    let mut call = call;
    call.set_parser_info("import_pattern".to_string(), import_pattern_expr);

    (
        Pipeline::from_vec(vec![Expression::new(
            working_set,
            Expr::Call(call),
            Span::concat(spans),
            Type::Any,
        )]),
        exportables,
    )
}

pub fn parse_hide(working_set: &mut StateWorkingSet, lite_command: &LiteCommand) -> Pipeline {
    let spans = &lite_command.parts;

    if working_set.get_span_contents(spans[0]) != b"hide" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for 'hide' command".into(),
            Span::concat(spans),
        ));
        return garbage_pipeline(working_set, spans);
    }
    if let Some(redirection) = lite_command.redirection.as_ref() {
        working_set.error(redirecting_builtin_error("hide", redirection));
        return garbage_pipeline(working_set, spans);
    }

    let (call, args_spans) = match working_set.find_decl(b"hide") {
        Some(decl_id) => {
            let ParsedInternalCall {
                call,
                output,
                call_kind,
            } = parse_internal_call(working_set, spans[0], &spans[1..], decl_id);

            if call_kind != CallKind::Valid {
                return Pipeline::from_vec(vec![Expression::new(
                    working_set,
                    Expr::Call(call),
                    Span::concat(spans),
                    output,
                )]);
            }

            (call, &spans[1..])
        }
        None => {
            working_set.error(ParseError::UnknownState(
                "internal error: 'hide' declaration not found".into(),
                Span::concat(spans),
            ));
            return garbage_pipeline(working_set, spans);
        }
    };

    let import_pattern_expr = parse_import_pattern(working_set, args_spans);

    let import_pattern = if let Expression {
        expr: Expr::ImportPattern(import_pattern),
        ..
    } = &import_pattern_expr
    {
        import_pattern.clone()
    } else {
        working_set.error(ParseError::UnknownState(
            "internal error: Import pattern positional is not import pattern".into(),
            import_pattern_expr.span,
        ));
        return garbage_pipeline(working_set, spans);
    };

    let bytes = working_set.get_span_contents(spans[0]);

    if bytes == b"hide" && spans.len() >= 2 {
        for span in spans[1..].iter() {
            parse_string(working_set, *span);
        }

        // module used only internally, not saved anywhere
        let (is_module, module) =
            if let Some(module_id) = working_set.find_module(&import_pattern.head.name) {
                (true, working_set.get_module(module_id).clone())
            } else if import_pattern.members.is_empty() {
                // The pattern head can be:
                if let Some(id) = working_set.find_decl(&import_pattern.head.name) {
                    // a custom command,
                    let mut module = Module::new(b"tmp".to_vec());
                    module.add_decl(import_pattern.head.name.clone(), id);

                    (false, module)
                } else {
                    // , or it could be an env var (handled by the engine)
                    (false, Module::new(b"tmp".to_vec()))
                }
            } else {
                working_set.error(ParseError::ModuleNotFound(
                    spans[1],
                    String::from_utf8_lossy(&import_pattern.head.name).to_string(),
                ));
                return garbage_pipeline(working_set, spans);
            };

        // This kind of inverts the import pattern matching found in parse_use()
        let decls_to_hide = if import_pattern.members.is_empty() {
            if is_module {
                module.decl_names_with_head(&import_pattern.head.name)
            } else {
                module.decl_names()
            }
        } else {
            match &import_pattern.members[0] {
                ImportPatternMember::Glob { .. } => module.decl_names(),
                ImportPatternMember::Name { name, span } => {
                    let mut decls = vec![];

                    if name == b"main" {
                        if module.main.is_some() {
                            decls.push(import_pattern.head.name.clone());
                        } else {
                            working_set.error(ParseError::ExportNotFound(*span));
                        }
                    } else if let Some(item) =
                        module.decl_name_with_head(name, &import_pattern.head.name)
                    {
                        decls.push(item);
                    } else {
                        working_set.error(ParseError::ExportNotFound(*span));
                    }

                    decls
                }
                ImportPatternMember::List { names } => {
                    let mut decls = vec![];

                    for (name, span) in names {
                        if name == b"main" {
                            if module.main.is_some() {
                                decls.push(import_pattern.head.name.clone());
                            } else {
                                working_set.error(ParseError::ExportNotFound(*span));
                                break;
                            }
                        } else if let Some(item) =
                            module.decl_name_with_head(name, &import_pattern.head.name)
                        {
                            decls.push(item);
                        } else {
                            working_set.error(ParseError::ExportNotFound(*span));
                            break;
                        }
                    }

                    decls
                }
            }
        };

        let import_pattern = {
            let decls: HashSet<Vec<u8>> = decls_to_hide.iter().cloned().collect();

            import_pattern.with_hidden(decls)
        };

        // TODO: `use spam; use spam foo; hide foo` will hide both `foo` and `spam foo` since
        // they point to the same DeclId. Do we want to keep it that way?
        working_set.hide_decls(&decls_to_hide);

        // Create a new Use command call to pass the new import pattern
        let import_pattern_expr = Expression::new(
            working_set,
            Expr::ImportPattern(Box::new(import_pattern)),
            Span::concat(args_spans),
            Type::Any,
        );

        let mut call = call;
        call.set_parser_info("import_pattern".to_string(), import_pattern_expr);

        Pipeline::from_vec(vec![Expression::new(
            working_set,
            Expr::Call(call),
            Span::concat(spans),
            Type::Any,
        )])
    } else {
        working_set.error(ParseError::UnknownState(
            "Expected structure: hide <name>".into(),
            Span::concat(spans),
        ));
        garbage_pipeline(working_set, spans)
    }
}

pub fn parse_overlay_new(working_set: &mut StateWorkingSet, call: Box<Call>) -> Pipeline {
    let call_span = call.span();

    let (overlay_name, _) = if let Some(expr) = call.positional_nth(0) {
        match eval_constant(working_set, expr) {
            Ok(val) => match val.coerce_into_string() {
                Ok(s) => (s, expr.span),
                Err(err) => {
                    working_set.error(err.wrap(working_set, call_span));
                    return garbage_pipeline(working_set, &[call_span]);
                }
            },
            Err(err) => {
                working_set.error(err.wrap(working_set, call_span));
                return garbage_pipeline(working_set, &[call_span]);
            }
        }
    } else {
        working_set.error(ParseError::UnknownState(
            "internal error: Missing required positional after call parsing".into(),
            call_span,
        ));
        return garbage_pipeline(working_set, &[call_span]);
    };

    let pipeline = Pipeline::from_vec(vec![Expression::new(
        working_set,
        Expr::Call(call),
        call_span,
        Type::Any,
    )]);

    let module_id = working_set.add_module(
        &overlay_name,
        Module::new(overlay_name.as_bytes().to_vec()),
        vec![],
    );

    working_set.add_overlay(
        overlay_name.as_bytes().to_vec(),
        module_id,
        ResolvedImportPattern::new(vec![], vec![], vec![], vec![]),
        false,
    );

    pipeline
}

pub fn parse_overlay_use(working_set: &mut StateWorkingSet, call: Box<Call>) -> Pipeline {
    let call_span = call.span();

    let (overlay_name, overlay_name_span) = if let Some(expr) = call.positional_nth(0) {
        match eval_constant(working_set, expr) {
            Ok(Value::Nothing { .. }) => {
                let mut call = call;
                call.set_parser_info(
                    "noop".to_string(),
                    Expression::new_unknown(Expr::Bool(true), Span::unknown(), Type::Bool),
                );
                return Pipeline::from_vec(vec![Expression::new(
                    working_set,
                    Expr::Call(call),
                    call_span,
                    Type::Any,
                )]);
            }
            Ok(val) => match val.coerce_into_string() {
                Ok(s) => (s, expr.span),
                Err(err) => {
                    working_set.error(err.wrap(working_set, call_span));
                    return garbage_pipeline(working_set, &[call_span]);
                }
            },
            Err(err) => {
                working_set.error(err.wrap(working_set, call_span));
                return garbage_pipeline(working_set, &[call_span]);
            }
        }
    } else {
        working_set.error(ParseError::UnknownState(
            "internal error: Missing required positional after call parsing".into(),
            call_span,
        ));
        return garbage_pipeline(working_set, &[call_span]);
    };

    let new_name = if let Some(kw_expression) = call.positional_nth(1) {
        if let Some(new_name_expression) = kw_expression.as_keyword() {
            match eval_constant(working_set, new_name_expression) {
                Ok(val) => match val.coerce_into_string() {
                    Ok(s) => Some(Spanned {
                        item: s,
                        span: new_name_expression.span,
                    }),
                    Err(err) => {
                        working_set.error(err.wrap(working_set, call_span));
                        return garbage_pipeline(working_set, &[call_span]);
                    }
                },
                Err(err) => {
                    working_set.error(err.wrap(working_set, call_span));
                    return garbage_pipeline(working_set, &[call_span]);
                }
            }
        } else {
            working_set.error(ParseError::ExpectedKeyword(
                "as keyword".to_string(),
                kw_expression.span,
            ));
            return garbage_pipeline(working_set, &[call_span]);
        }
    } else {
        None
    };

    let Ok(has_prefix) = has_flag_const(working_set, &call, "prefix") else {
        return garbage_pipeline(working_set, &[call_span]);
    };
    let Ok(do_reload) = has_flag_const(working_set, &call, "reload") else {
        return garbage_pipeline(working_set, &[call_span]);
    };

    let pipeline = Pipeline::from_vec(vec![Expression::new(
        working_set,
        Expr::Call(call.clone()),
        call_span,
        Type::Any,
    )]);

    let (final_overlay_name, origin_module, origin_module_id, is_module_updated) =
        if let Some(overlay_frame) = working_set.find_overlay(overlay_name.as_bytes()) {
            // Activate existing overlay

            // First, check for errors
            if has_prefix && !overlay_frame.prefixed {
                working_set.error(ParseError::OverlayPrefixMismatch(
                    overlay_name,
                    "without".to_string(),
                    overlay_name_span,
                ));
                return pipeline;
            }

            if !has_prefix && overlay_frame.prefixed {
                working_set.error(ParseError::OverlayPrefixMismatch(
                    overlay_name,
                    "with".to_string(),
                    overlay_name_span,
                ));
                return pipeline;
            }

            if let Some(new_name) = new_name
                && new_name.item != overlay_name
            {
                working_set.error(ParseError::CantAddOverlayHelp(
                    format!(
                        "Cannot add overlay as '{}' because it already exists under the name '{}'",
                        new_name.item, overlay_name
                    ),
                    new_name.span,
                ));
                return pipeline;
            }

            let module_id = overlay_frame.origin;

            if let Some(new_module_id) = working_set.find_module(overlay_name.as_bytes()) {
                if !do_reload && (module_id == new_module_id) {
                    (
                        overlay_name,
                        Module::new(working_set.get_module(module_id).name.clone()),
                        module_id,
                        false,
                    )
                } else {
                    // The origin module of an overlay changed => update it
                    (
                        overlay_name,
                        working_set.get_module(new_module_id).clone(),
                        new_module_id,
                        true,
                    )
                }
            } else {
                let module_name = overlay_name.as_bytes().to_vec();
                (overlay_name, Module::new(module_name), module_id, true)
            }
        } else {
            // Create a new overlay
            if let Some(module_id) =
                // the name is a module
                working_set.find_module(overlay_name.as_bytes())
            {
                (
                    new_name.map(|spanned| spanned.item).unwrap_or(overlay_name),
                    working_set.get_module(module_id).clone(),
                    module_id,
                    true,
                )
            } else if let Some(module_id) = parse_module_file_or_dir(
                working_set,
                overlay_name.as_bytes(),
                overlay_name_span,
                new_name.as_ref().map(|spanned| spanned.item.clone()),
            ) {
                // try file or directory
                let new_module = working_set.get_module(module_id).clone();
                (
                    new_name
                        .map(|spanned| spanned.item)
                        .unwrap_or_else(|| String::from_utf8_lossy(&new_module.name).to_string()),
                    new_module,
                    module_id,
                    true,
                )
            } else {
                working_set.error(ParseError::ModuleOrOverlayNotFound(overlay_name_span));
                return pipeline;
            }
        };

    let (definitions, errors) = if is_module_updated {
        if has_prefix {
            origin_module.resolve_import_pattern(
                working_set,
                origin_module_id,
                &[],
                Some(final_overlay_name.as_bytes()),
                call.head,
                &mut vec![],
            )
        } else {
            origin_module.resolve_import_pattern(
                working_set,
                origin_module_id,
                &[ImportPatternMember::Glob {
                    span: overlay_name_span,
                }],
                Some(final_overlay_name.as_bytes()),
                call.head,
                &mut vec![],
            )
        }
    } else {
        (
            ResolvedImportPattern::new(vec![], vec![], vec![], vec![]),
            vec![],
        )
    };

    if errors.is_empty() {
        working_set.add_overlay(
            final_overlay_name.as_bytes().to_vec(),
            origin_module_id,
            definitions,
            has_prefix,
        );
    } else {
        working_set.parse_errors.extend(errors);
    }

    // Change the call argument to include the Overlay expression with the module ID
    let mut call = call;
    call.set_parser_info(
        "overlay_expr".to_string(),
        Expression::new(
            working_set,
            Expr::Overlay(if is_module_updated {
                Some(origin_module_id)
            } else {
                None
            }),
            overlay_name_span,
            Type::Any,
        ),
    );

    Pipeline::from_vec(vec![Expression::new(
        working_set,
        Expr::Call(call),
        call_span,
        Type::Any,
    )])
}

pub fn parse_overlay_hide(working_set: &mut StateWorkingSet, call: Box<Call>) -> Pipeline {
    let call_span = call.span();

    let (overlay_name, overlay_name_span) = if let Some(expr) = call.positional_nth(0) {
        match eval_constant(working_set, expr) {
            Ok(val) => match val.coerce_into_string() {
                Ok(s) => (s, expr.span),
                Err(err) => {
                    working_set.error(err.wrap(working_set, call_span));
                    return garbage_pipeline(working_set, &[call_span]);
                }
            },
            Err(err) => {
                working_set.error(err.wrap(working_set, call_span));
                return garbage_pipeline(working_set, &[call_span]);
            }
        }
    } else {
        (
            String::from_utf8_lossy(working_set.last_overlay_name()).to_string(),
            call_span,
        )
    };

    let Ok(keep_custom) = has_flag_const(working_set, &call, "keep-custom") else {
        return garbage_pipeline(working_set, &[call_span]);
    };

    let pipeline = Pipeline::from_vec(vec![Expression::new(
        working_set,
        Expr::Call(call),
        call_span,
        Type::Any,
    )]);

    if overlay_name == DEFAULT_OVERLAY_NAME {
        working_set.error(ParseError::CantHideDefaultOverlay(
            overlay_name,
            overlay_name_span,
        ));

        return pipeline;
    }

    if !working_set
        .unique_overlay_names()
        .contains(&overlay_name.as_bytes())
    {
        working_set.error(ParseError::ActiveOverlayNotFound(overlay_name_span));
        return pipeline;
    }

    if working_set.num_overlays() < 2 {
        working_set.error(ParseError::CantRemoveLastOverlay(overlay_name_span));
        return pipeline;
    }

    working_set.remove_overlay(overlay_name.as_bytes(), keep_custom);

    pipeline
}

pub fn parse_let(working_set: &mut StateWorkingSet, spans: &[Span]) -> Pipeline {
    trace!("parsing: let");

    // JT: Disabling check_name because it doesn't work with optional types in the declaration
    // if let Some(span) = check_name(working_set, spans) {
    //     return Pipeline::from_vec(vec![garbage(*span)]);
    // }

    if let Some(decl_id) = working_set.find_decl(b"let") {
        if spans.len() >= 4 {
            // This is a bit of by-hand parsing to get around the issue where we want to parse in the reverse order
            // so that the var-id created by the variable isn't visible in the expression that init it
            for span in spans.iter().enumerate() {
                let item = working_set.get_span_contents(*span.1);
                // https://github.com/nushell/nushell/issues/9596, let = if $
                // let x = 'f', = at least start from index 2
                if item == b"=" && spans.len() > (span.0 + 1) && span.0 > 1 {
                    let (tokens, parse_error) = lex(
                        working_set.get_span_contents(Span::concat(&spans[(span.0 + 1)..])),
                        spans[span.0 + 1].start,
                        &[],
                        &[],
                        false,
                    );

                    if let Some(parse_error) = parse_error {
                        working_set.error(parse_error)
                    }

                    let rvalue_span = Span::concat(&spans[(span.0 + 1)..]);
                    let rvalue_block = parse_block(working_set, &tokens, rvalue_span, false, true);

                    let output_type = rvalue_block.output_type();

                    let block_id = working_set.add_block(Arc::new(rvalue_block));

                    let rvalue = Expression::new(
                        working_set,
                        Expr::Block(block_id),
                        rvalue_span,
                        output_type,
                    );

                    let mut idx = 0;
                    let (lvalue, explicit_type) =
                        parse_var_with_opt_type(working_set, &spans[1..(span.0)], &mut idx, false);
                    // check for extra tokens after the identifier
                    if idx + 1 < span.0 - 1 {
                        working_set.error(ParseError::ExtraTokens(spans[idx + 2]));
                    }

                    let var_name =
                        String::from_utf8_lossy(working_set.get_span_contents(lvalue.span))
                            .trim_start_matches('$')
                            .to_string();

                    if RESERVED_VARIABLE_NAMES.contains(&var_name.as_str()) {
                        working_set.error(ParseError::NameIsBuiltinVar(var_name, lvalue.span))
                    }

                    let var_id = lvalue.as_var();
                    let rhs_type = rvalue.ty.clone();

                    if let Some(explicit_type) = &explicit_type
                        && !type_compatible(explicit_type, &rhs_type)
                    {
                        working_set.error(ParseError::TypeMismatch(
                            explicit_type.clone(),
                            rhs_type.clone(),
                            Span::concat(&spans[(span.0 + 1)..]),
                        ));
                    }

                    if let Some(var_id) = var_id
                        && explicit_type.is_none()
                    {
                        working_set.set_variable_type(var_id, rhs_type);
                    }

                    let call = Box::new(Call {
                        decl_id,
                        head: spans[0],
                        arguments: vec![Argument::Positional(lvalue), Argument::Positional(rvalue)],
                        parser_info: HashMap::new(),
                    });

                    return Pipeline::from_vec(vec![Expression::new(
                        working_set,
                        Expr::Call(call),
                        Span::concat(spans),
                        Type::Any,
                    )]);
                }
            }
        }
        let ParsedInternalCall { call, output, .. } =
            parse_internal_call(working_set, spans[0], &spans[1..], decl_id);

        return Pipeline::from_vec(vec![Expression::new(
            working_set,
            Expr::Call(call),
            Span::concat(spans),
            output,
        )]);
    } else {
        working_set.error(ParseError::UnknownState(
            "internal error: let or const statements not found in core language".into(),
            Span::concat(spans),
        ))
    }

    working_set.error(ParseError::UnknownState(
        "internal error: let or const statement unparsable".into(),
        Span::concat(spans),
    ));

    garbage_pipeline(working_set, spans)
}

/// Additionally returns a span encompassing the variable name, if successful.
pub fn parse_const(working_set: &mut StateWorkingSet, spans: &[Span]) -> (Pipeline, Option<Span>) {
    trace!("parsing: const");

    // JT: Disabling check_name because it doesn't work with optional types in the declaration
    // if let Some(span) = check_name(working_set, spans) {
    //     return Pipeline::from_vec(vec![garbage(working_set, *span)]);
    // }

    if let Some(decl_id) = working_set.find_decl(b"const") {
        if spans.len() >= 4 {
            // This is a bit of by-hand parsing to get around the issue where we want to parse in the reverse order
            // so that the var-id created by the variable isn't visible in the expression that init it
            for span in spans.iter().enumerate() {
                let item = working_set.get_span_contents(*span.1);
                // const x = 'f', = at least start from index 2
                if item == b"=" && spans.len() > (span.0 + 1) && span.0 > 1 {
                    // Parse the rvalue as a subexpression
                    let rvalue_span = Span::concat(&spans[(span.0 + 1)..]);

                    let (rvalue_tokens, rvalue_error) = lex(
                        working_set.get_span_contents(rvalue_span),
                        rvalue_span.start,
                        &[],
                        &[],
                        false,
                    );
                    working_set.parse_errors.extend(rvalue_error);

                    trace!("parsing: const right-hand side subexpression");
                    let rvalue_block =
                        parse_block(working_set, &rvalue_tokens, rvalue_span, false, true);
                    let rvalue_ty = rvalue_block.output_type();
                    let rvalue_block_id = working_set.add_block(Arc::new(rvalue_block));
                    let rvalue = Expression::new(
                        working_set,
                        Expr::Subexpression(rvalue_block_id),
                        rvalue_span,
                        rvalue_ty,
                    );

                    let mut idx = 0;

                    let (lvalue, explicit_type) =
                        parse_var_with_opt_type(working_set, &spans[1..(span.0)], &mut idx, false);
                    // check for extra tokens after the identifier
                    if idx + 1 < span.0 - 1 {
                        working_set.error(ParseError::ExtraTokens(spans[idx + 2]));
                    }

                    let var_name =
                        String::from_utf8_lossy(working_set.get_span_contents(lvalue.span))
                            .trim_start_matches('$')
                            .to_string();

                    if RESERVED_VARIABLE_NAMES.contains(&var_name.as_str()) {
                        working_set.error(ParseError::NameIsBuiltinVar(var_name, lvalue.span))
                    }

                    let var_id = lvalue.as_var();
                    let rhs_type = rvalue.ty.clone();

                    if let Some(explicit_type) = &explicit_type
                        && !type_compatible(explicit_type, &rhs_type)
                    {
                        working_set.error(ParseError::TypeMismatch(
                            explicit_type.clone(),
                            rhs_type.clone(),
                            Span::concat(&spans[(span.0 + 1)..]),
                        ));
                    }

                    if let Some(var_id) = var_id {
                        if explicit_type.is_none() {
                            working_set.set_variable_type(var_id, rhs_type);
                        }

                        match eval_constant(working_set, &rvalue) {
                            Ok(mut value) => {
                                // In case rhs is parsed as 'any' but is evaluated to a concrete
                                // type:
                                let mut const_type = value.get_type();

                                if let Some(explicit_type) = &explicit_type {
                                    if !type_compatible(explicit_type, &const_type) {
                                        working_set.error(ParseError::TypeMismatch(
                                            explicit_type.clone(),
                                            const_type.clone(),
                                            Span::concat(&spans[(span.0 + 1)..]),
                                        ));
                                    }
                                    let val_span = value.span();

                                    // need to convert to Value::glob if rhs is string, and
                                    // the const variable is annotated with glob type.
                                    match value {
                                        Value::String { val, .. }
                                            if explicit_type == &Type::Glob =>
                                        {
                                            value = Value::glob(val, false, val_span);
                                            const_type = value.get_type();
                                        }
                                        _ => {}
                                    }
                                }

                                working_set.set_variable_type(var_id, const_type);

                                // Assign the constant value to the variable
                                working_set.set_variable_const_val(var_id, value);
                            }
                            Err(err) => working_set.error(err.wrap(working_set, rvalue.span)),
                        }
                    }

                    let call = Box::new(Call {
                        decl_id,
                        head: spans[0],
                        arguments: vec![
                            Argument::Positional(lvalue.clone()),
                            Argument::Positional(rvalue),
                        ],
                        parser_info: HashMap::new(),
                    });

                    return (
                        Pipeline::from_vec(vec![Expression::new(
                            working_set,
                            Expr::Call(call),
                            Span::concat(spans),
                            Type::Any,
                        )]),
                        Some(lvalue.span),
                    );
                }
            }
        }
        let ParsedInternalCall { call, output, .. } =
            parse_internal_call(working_set, spans[0], &spans[1..], decl_id);

        return (
            Pipeline::from_vec(vec![Expression::new(
                working_set,
                Expr::Call(call),
                Span::concat(spans),
                output,
            )]),
            None,
        );
    } else {
        working_set.error(ParseError::UnknownState(
            "internal error: let or const statements not found in core language".into(),
            Span::concat(spans),
        ))
    }

    working_set.error(ParseError::UnknownState(
        "internal error: let or const statement unparsable".into(),
        Span::concat(spans),
    ));

    (garbage_pipeline(working_set, spans), None)
}

pub fn parse_mut(working_set: &mut StateWorkingSet, spans: &[Span]) -> Pipeline {
    trace!("parsing: mut");

    // JT: Disabling check_name because it doesn't work with optional types in the declaration
    // if let Some(span) = check_name(working_set, spans) {
    //     return Pipeline::from_vec(vec![garbage(working_set, *span)]);
    // }

    if let Some(decl_id) = working_set.find_decl(b"mut") {
        if spans.len() >= 4 {
            // This is a bit of by-hand parsing to get around the issue where we want to parse in the reverse order
            // so that the var-id created by the variable isn't visible in the expression that init it
            for span in spans.iter().enumerate() {
                let item = working_set.get_span_contents(*span.1);
                // mut x = 'f', = at least start from index 2
                if item == b"=" && spans.len() > (span.0 + 1) && span.0 > 1 {
                    let (tokens, parse_error) = lex(
                        working_set.get_span_contents(Span::concat(&spans[(span.0 + 1)..])),
                        spans[span.0 + 1].start,
                        &[],
                        &[],
                        false,
                    );

                    if let Some(parse_error) = parse_error {
                        working_set.error(parse_error);
                    }

                    let rvalue_span = Span::concat(&spans[(span.0 + 1)..]);
                    let rvalue_block = parse_block(working_set, &tokens, rvalue_span, false, true);

                    let output_type = rvalue_block.output_type();

                    let block_id = working_set.add_block(Arc::new(rvalue_block));

                    let rvalue = Expression::new(
                        working_set,
                        Expr::Block(block_id),
                        rvalue_span,
                        output_type,
                    );

                    let mut idx = 0;

                    let (lvalue, explicit_type) =
                        parse_var_with_opt_type(working_set, &spans[1..(span.0)], &mut idx, true);
                    // check for extra tokens after the identifier
                    if idx + 1 < span.0 - 1 {
                        working_set.error(ParseError::ExtraTokens(spans[idx + 2]));
                    }

                    let var_name =
                        String::from_utf8_lossy(working_set.get_span_contents(lvalue.span))
                            .trim_start_matches('$')
                            .to_string();

                    if RESERVED_VARIABLE_NAMES.contains(&var_name.as_str()) {
                        working_set.error(ParseError::NameIsBuiltinVar(var_name, lvalue.span))
                    }

                    let var_id = lvalue.as_var();
                    let rhs_type = rvalue.ty.clone();

                    if let Some(explicit_type) = &explicit_type
                        && !type_compatible(explicit_type, &rhs_type)
                    {
                        working_set.error(ParseError::TypeMismatch(
                            explicit_type.clone(),
                            rhs_type.clone(),
                            Span::concat(&spans[(span.0 + 1)..]),
                        ));
                    }

                    if let Some(var_id) = var_id
                        && explicit_type.is_none()
                    {
                        working_set.set_variable_type(var_id, rhs_type);
                    }

                    let call = Box::new(Call {
                        decl_id,
                        head: spans[0],
                        arguments: vec![Argument::Positional(lvalue), Argument::Positional(rvalue)],
                        parser_info: HashMap::new(),
                    });

                    return Pipeline::from_vec(vec![Expression::new(
                        working_set,
                        Expr::Call(call),
                        Span::concat(spans),
                        Type::Any,
                    )]);
                }
            }
        }
        let ParsedInternalCall { call, output, .. } =
            parse_internal_call(working_set, spans[0], &spans[1..], decl_id);

        return Pipeline::from_vec(vec![Expression::new(
            working_set,
            Expr::Call(call),
            Span::concat(spans),
            output,
        )]);
    } else {
        working_set.error(ParseError::UnknownState(
            "internal error: let or const statements not found in core language".into(),
            Span::concat(spans),
        ))
    }

    working_set.error(ParseError::UnknownState(
        "internal error: let or const statement unparsable".into(),
        Span::concat(spans),
    ));

    garbage_pipeline(working_set, spans)
}

pub fn parse_source(working_set: &mut StateWorkingSet, lite_command: &LiteCommand) -> Pipeline {
    trace!("parsing source");
    let spans = &lite_command.parts;
    let name = working_set.get_span_contents(spans[0]);

    if name == b"source" || name == b"source-env" {
        if let Some(redirection) = lite_command.redirection.as_ref() {
            let name = if name == b"source" {
                "source"
            } else {
                "source-env"
            };
            working_set.error(redirecting_builtin_error(name, redirection));
            return garbage_pipeline(working_set, spans);
        }

        let scoped = name == b"source-env";

        if let Some(decl_id) = working_set.find_decl(name) {
            #[allow(deprecated)]
            let cwd = working_set.get_cwd();

            // Is this the right call to be using here?
            // Some of the others (`parse_let`) use it, some of them (`parse_hide`) don't.
            let ParsedInternalCall {
                call,
                output,
                call_kind,
            } = parse_internal_call(working_set, spans[0], &spans[1..], decl_id);

            if call_kind == CallKind::Help {
                return Pipeline::from_vec(vec![Expression::new(
                    working_set,
                    Expr::Call(call),
                    Span::concat(spans),
                    output,
                )]);
            }

            // Command and one file name
            if spans.len() >= 2 {
                let expr = parse_value(working_set, spans[1], &SyntaxShape::Any);

                let val = match eval_constant(working_set, &expr) {
                    Ok(val) => val,
                    Err(err) => {
                        working_set.error(err.wrap(working_set, Span::concat(&spans[1..])));
                        return Pipeline::from_vec(vec![Expression::new(
                            working_set,
                            Expr::Call(call),
                            Span::concat(&spans[1..]),
                            Type::Any,
                        )]);
                    }
                };

                if val.is_nothing() {
                    let mut call = call;
                    call.set_parser_info(
                        "noop".to_string(),
                        Expression::new_unknown(Expr::Nothing, Span::unknown(), Type::Nothing),
                    );
                    return Pipeline::from_vec(vec![Expression::new(
                        working_set,
                        Expr::Call(call),
                        Span::concat(spans),
                        Type::Any,
                    )]);
                }

                let filename = match val.coerce_into_string() {
                    Ok(s) => s,
                    Err(err) => {
                        working_set.error(err.wrap(working_set, Span::concat(&spans[1..])));
                        return Pipeline::from_vec(vec![Expression::new(
                            working_set,
                            Expr::Call(call),
                            Span::concat(&spans[1..]),
                            Type::Any,
                        )]);
                    }
                };

                if let Some(path) = find_in_dirs(&filename, working_set, &cwd, Some(LIB_DIRS_VAR)) {
                    if let Some(contents) = path.read(working_set) {
                        // Add the file to the stack of files being processed.
                        if let Err(e) = working_set.files.push(path.clone().path_buf(), spans[1]) {
                            working_set.error(e);
                            return garbage_pipeline(working_set, spans);
                        }

                        // This will load the defs from the file into the
                        // working set, if it was a successful parse.
                        let mut block = parse(
                            working_set,
                            Some(&path.path().to_string_lossy()),
                            &contents,
                            scoped,
                        );
                        if block.ir_block.is_none() {
                            let block_mut = Arc::make_mut(&mut block);
                            compile_block(working_set, block_mut);
                        }

                        // Remove the file from the stack of files being processed.
                        working_set.files.pop();

                        // Save the block into the working set
                        let block_id = working_set.add_block(block);

                        let mut call_with_block = call;

                        // FIXME: Adding this expression to the positional creates a syntax highlighting error
                        // after writing `source example.nu`
                        call_with_block.set_parser_info(
                            "block_id".to_string(),
                            Expression::new(
                                working_set,
                                Expr::Int(block_id.get() as i64),
                                spans[1],
                                Type::Any,
                            ),
                        );

                        // store the file path as a string to be gathered later
                        call_with_block.set_parser_info(
                            "block_id_name".to_string(),
                            Expression::new(
                                working_set,
                                Expr::Filepath(path.path_buf().display().to_string(), false),
                                spans[1],
                                Type::String,
                            ),
                        );

                        return Pipeline::from_vec(vec![Expression::new(
                            working_set,
                            Expr::Call(call_with_block),
                            Span::concat(spans),
                            Type::Any,
                        )]);
                    }
                } else {
                    working_set.error(ParseError::SourcedFileNotFound(filename, spans[1]));
                }
            }
            return Pipeline::from_vec(vec![Expression::new(
                working_set,
                Expr::Call(call),
                Span::concat(spans),
                Type::Any,
            )]);
        }
    }
    working_set.error(ParseError::UnknownState(
        "internal error: source statement unparsable".into(),
        Span::concat(spans),
    ));
    garbage_pipeline(working_set, spans)
}

pub fn parse_where_expr(working_set: &mut StateWorkingSet, spans: &[Span]) -> Expression {
    trace!("parsing: where");

    if !spans.is_empty() && working_set.get_span_contents(spans[0]) != b"where" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for 'where' command".into(),
            Span::concat(spans),
        ));
        return garbage(working_set, Span::concat(spans));
    }

    if spans.len() < 2 {
        working_set.error(ParseError::MissingPositional(
            "row condition".into(),
            Span::concat(spans),
            "where <row_condition>".into(),
        ));
        return garbage(working_set, Span::concat(spans));
    }

    let call = match working_set.find_decl(b"where") {
        Some(decl_id) => {
            let ParsedInternalCall {
                call,
                output,
                call_kind,
            } = parse_internal_call(working_set, spans[0], &spans[1..], decl_id);

            if call_kind != CallKind::Valid {
                return Expression::new(working_set, Expr::Call(call), Span::concat(spans), output);
            }

            call
        }
        None => {
            working_set.error(ParseError::UnknownState(
                "internal error: 'where' declaration not found".into(),
                Span::concat(spans),
            ));
            return garbage(working_set, Span::concat(spans));
        }
    };

    Expression::new(
        working_set,
        Expr::Call(call),
        Span::concat(spans),
        Type::Any,
    )
}

pub fn parse_where(working_set: &mut StateWorkingSet, lite_command: &LiteCommand) -> Pipeline {
    let expr = parse_where_expr(working_set, &lite_command.parts);
    let redirection = lite_command
        .redirection
        .as_ref()
        .map(|r| parse_redirection(working_set, r));

    let element = PipelineElement {
        pipe: None,
        expr,
        redirection,
    };

    Pipeline {
        elements: vec![element],
    }
}

#[cfg(feature = "plugin")]
pub fn parse_plugin_use(working_set: &mut StateWorkingSet, call: Box<Call>) -> Pipeline {
    use nu_protocol::{FromValue, PluginRegistryFile};

    #[allow(deprecated)]
    let cwd = working_set.get_cwd();

    if let Err(err) = (|| {
        let name = call
            .positional_nth(0)
            .map(|expr| {
                eval_constant(working_set, expr)
                    .and_then(Spanned::<String>::from_value)
                    .map_err(|err| err.wrap(working_set, call.head))
            })
            .expect("required positional should have been checked")?;

        let plugin_config = call
            .named_iter()
            .find(|(arg_name, _, _)| arg_name.item == "plugin-config")
            .map(|(_, _, expr)| {
                let expr = expr
                    .as_ref()
                    .expect("--plugin-config arg should have been checked already");
                eval_constant(working_set, expr)
                    .and_then(Spanned::<String>::from_value)
                    .map_err(|err| err.wrap(working_set, call.head))
            })
            .transpose()?;

        // The name could also be a filename, so try our best to expand it for that match.
        let filename_query = {
            let path = nu_path::expand_path_with(&name.item, &cwd, true);
            path.to_str()
                .and_then(|path_str| {
                    find_in_dirs(path_str, working_set, &cwd, Some("NU_PLUGIN_DIRS"))
                })
                .map(|parser_path| parser_path.path_buf())
                .unwrap_or(path)
        };

        // Find the actual plugin config path location. We don't have a const/env variable for this,
        // it either lives in the current working directory or in the script's directory
        let plugin_config_path = if let Some(custom_path) = &plugin_config {
            find_in_dirs(&custom_path.item, working_set, &cwd, None).ok_or_else(|| {
                ParseError::FileNotFound(custom_path.item.clone(), custom_path.span)
            })?
        } else {
            ParserPath::RealPath(
                working_set
                    .permanent_state
                    .plugin_path
                    .as_ref()
                    .ok_or_else(|| ParseError::LabeledErrorWithHelp {
                        error: "Plugin registry file not set".into(),
                        label: "can't load plugin without registry file".into(),
                        span: call.head,
                        help:
                            "pass --plugin-config to `plugin use` when $nu.plugin-path is not set"
                                .into(),
                    })?
                    .to_owned(),
            )
        };

        let file = plugin_config_path.open(working_set).map_err(|err| {
            ParseError::LabeledError(
                "Plugin registry file can't be opened".into(),
                err.to_string(),
                plugin_config.as_ref().map(|p| p.span).unwrap_or(call.head),
            )
        })?;

        // The file is now open, so we just have to parse the contents and find the plugin
        let contents = PluginRegistryFile::read_from(file, Some(call.head))
            .map_err(|err| err.wrap(working_set, call.head))?;

        let plugin_item = contents
            .plugins
            .iter()
            .find(|plugin| plugin.name == name.item || plugin.filename == filename_query)
            .ok_or_else(|| ParseError::PluginNotFound {
                name: name.item.clone(),
                name_span: name.span,
                plugin_config_span: plugin_config.as_ref().map(|p| p.span),
            })?;

        // Now add the signatures to the working set
        nu_plugin_engine::load_plugin_registry_item(working_set, plugin_item, Some(call.head))
            .map_err(|err| err.wrap(working_set, call.head))?;

        Ok(())
    })() {
        working_set.error(err);
    }

    let call_span = call.span();

    Pipeline::from_vec(vec![Expression::new(
        working_set,
        Expr::Call(call),
        call_span,
        Type::Nothing,
    )])
}

pub fn find_dirs_var(working_set: &StateWorkingSet, var_name: &str) -> Option<VarId> {
    working_set
        .find_variable(format!("${var_name}").as_bytes())
        .filter(|var_id| working_set.get_variable(*var_id).const_val.is_some())
}

/// This helper function is used to find files during parsing
///
/// First, the actual current working directory is selected as
///   a) the directory of a file currently being parsed
///   b) current working directory (PWD)
///
/// Then, if the file is not found in the actual cwd, dirs_var is checked.
/// For now, we first check for a const with the name of `dirs_var_name`,
/// and if that's not found, then we try to look for an environment variable of the same name.
/// If there is a relative path in dirs_var, it is assumed to be relative to the actual cwd
/// determined in the first step.
///
/// Always returns an absolute path
pub fn find_in_dirs(
    filename: &str,
    working_set: &StateWorkingSet,
    cwd: &str,
    dirs_var_name: Option<&str>,
) -> Option<ParserPath> {
    if is_windows_device_path(Path::new(&filename)) {
        return Some(ParserPath::RealPath(filename.into()));
    }

    pub fn find_in_dirs_with_id(
        filename: &str,
        working_set: &StateWorkingSet,
        cwd: &str,
        dirs_var_name: Option<&str>,
    ) -> Option<ParserPath> {
        // Choose whether to use file-relative or PWD-relative path
        let actual_cwd = working_set
            .files
            .current_working_directory()
            .unwrap_or(Path::new(cwd));

        // Try if we have an existing virtual path
        if let Some(virtual_path) = working_set.find_virtual_path(filename) {
            return Some(ParserPath::from_virtual_path(
                working_set,
                filename,
                virtual_path,
            ));
        } else {
            let abs_virtual_filename = actual_cwd.join(filename);
            let abs_virtual_filename = abs_virtual_filename.to_string_lossy();

            if let Some(virtual_path) = working_set.find_virtual_path(&abs_virtual_filename) {
                return Some(ParserPath::from_virtual_path(
                    working_set,
                    &abs_virtual_filename,
                    virtual_path,
                ));
            }
        }

        // Try if we have an existing physical path
        if let Ok(p) = canonicalize_with(filename, actual_cwd) {
            return Some(ParserPath::RealPath(p));
        }

        // Early-exit if path is non-existent absolute path
        let path = Path::new(filename);
        if !path.is_relative() {
            return None;
        }

        // Look up relative path from NU_LIB_DIRS
        dirs_var_name
            .as_ref()
            .and_then(|dirs_var_name| find_dirs_var(working_set, dirs_var_name))
            .map(|var_id| working_set.get_variable(var_id))?
            .const_val
            .as_ref()?
            .as_list()
            .ok()?
            .iter()
            .map(|lib_dir| -> Option<PathBuf> {
                let dir = lib_dir.to_path().ok()?;
                let dir_abs = canonicalize_with(dir, actual_cwd).ok()?;
                canonicalize_with(filename, dir_abs).ok()
            })
            .find(Option::is_some)
            .flatten()
            .map(ParserPath::RealPath)
    }

    // TODO: remove (see #8310)
    // Same as find_in_dirs_with_id but using $env.NU_LIB_DIRS instead of constant
    pub fn find_in_dirs_old(
        filename: &str,
        working_set: &StateWorkingSet,
        cwd: &str,
        dirs_env: Option<&str>,
    ) -> Option<PathBuf> {
        // Choose whether to use file-relative or PWD-relative path
        let actual_cwd = working_set
            .files
            .current_working_directory()
            .unwrap_or(Path::new(cwd));

        if let Ok(p) = canonicalize_with(filename, actual_cwd) {
            Some(p)
        } else {
            let path = Path::new(filename);

            if path.is_relative() {
                if let Some(lib_dirs) =
                    dirs_env.and_then(|dirs_env| working_set.get_env_var(dirs_env))
                {
                    if let Ok(dirs) = lib_dirs.as_list() {
                        for lib_dir in dirs {
                            if let Ok(dir) = lib_dir.to_path() {
                                // make sure the dir is absolute path
                                if let Ok(dir_abs) = canonicalize_with(dir, actual_cwd)
                                    && let Ok(path) = canonicalize_with(filename, dir_abs)
                                {
                                    return Some(path);
                                }
                            }
                        }

                        None
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
    }

    find_in_dirs_with_id(filename, working_set, cwd, dirs_var_name).or_else(|| {
        find_in_dirs_old(filename, working_set, cwd, dirs_var_name).map(ParserPath::RealPath)
    })
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

/// Run has_flag_const and push possible error to working_set
fn has_flag_const(working_set: &mut StateWorkingSet, call: &Call, name: &str) -> Result<bool, ()> {
    call.has_flag_const(working_set, name).map_err(|err| {
        working_set.error(err.wrap(working_set, call.span()));
    })
}
