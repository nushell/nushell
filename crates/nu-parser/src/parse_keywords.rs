use crate::{
    exportable::Exportable,
    parse_block,
    parser::{parse_redirection, redirecting_builtin_error},
    parser_path::ParserPath,
    type_check::{check_block_input_output, type_compatible},
};
use itertools::Itertools;
use log::trace;
use nu_path::canonicalize_with;
use nu_protocol::{
    ast::{
        Argument, Block, Call, Expr, Expression, ImportPattern, ImportPatternHead,
        ImportPatternMember, Pipeline, PipelineElement,
    },
    engine::{StateWorkingSet, DEFAULT_OVERLAY_NAME},
    eval_const::eval_constant,
    span, Alias, BlockId, DeclId, Module, ModuleId, ParseError, PositionalArg,
    ResolvedImportPattern, Span, Spanned, SyntaxShape, Type, Value, VarId,
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
    is_math_expression_like,
    known_external::KnownExternal,
    lex,
    lite_parser::{lite_parse, LiteCommand},
    parser::{
        check_call, garbage, garbage_pipeline, parse, parse_call, parse_expression,
        parse_full_signature, parse_import_pattern, parse_internal_call, parse_multispan_value,
        parse_string, parse_value, parse_var_with_opt_type, trim_quotes, ParsedInternalCall,
    },
    unescape_unquote_string, Token, TokenContents,
};

/// These parser keywords can be aliased
pub const ALIASABLE_PARSER_KEYWORDS: &[&[u8]] = &[b"overlay hide", b"overlay new", b"overlay use"];

pub const RESERVED_VARIABLE_NAMES: [&str; 3] = ["in", "nu", "env"];

/// These parser keywords cannot be aliased (either not possible, or support not yet added)
pub const UNALIASABLE_PARSER_KEYWORDS: &[&[u8]] = &[
    b"export",
    b"def",
    b"export def",
    b"for",
    b"extern",
    b"export extern",
    b"alias",
    b"export alias",
    b"export-env",
    b"module",
    b"use",
    b"export use",
    b"hide",
    // b"overlay",
    // b"overlay hide",
    // b"overlay new",
    // b"overlay use",
    b"let",
    b"const",
    b"mut",
    b"source",
    b"where",
    b"register",
    b"plugin use",
];

/// Check whether spans start with a parser keyword that can be aliased
pub fn is_unaliasable_parser_keyword(working_set: &StateWorkingSet, spans: &[Span]) -> bool {
    // try two words
    if let (Some(span1), Some(span2)) = (spans.first(), spans.get(1)) {
        let cmd_name = working_set.get_span_contents(span(&[*span1, *span2]));
        return UNALIASABLE_PARSER_KEYWORDS.contains(&cmd_name);
    }

    // try one word
    if let Some(span1) = spans.first() {
        let cmd_name = working_set.get_span_contents(*span1);
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
            return Pipeline::from_vec(vec![Expression {
                expr: Expr::Call(call),
                span: call_span,
                ty: Type::Any,
                custom_completion: None,
            }]);
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
    while pos < spans.len()
        && working_set
            .get_span_contents(spans[pos])
            .starts_with(&[b'-'])
    {
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
        if working_set
            .get_span_contents(spans[pos])
            .starts_with(&[b'['])
            || working_set
                .get_span_contents(spans[pos])
                .starts_with(&[b'('])
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
            span(spans),
        ));
        return garbage(spans[0]);
    }
    if let Some(redirection) = lite_command.redirection.as_ref() {
        working_set.error(redirecting_builtin_error("for", redirection));
        return garbage(spans[0]);
    }

    // Parsing the spans and checking that they match the register signature
    // Using a parsed call makes more sense than checking for how many spans are in the call
    // Also, by creating a call, it can be checked if it matches the declaration signature
    let (call, call_span) = match working_set.find_decl(b"for") {
        None => {
            working_set.error(ParseError::UnknownState(
                "internal error: for declaration not found".into(),
                span(spans),
            ));
            return garbage(spans[0]);
        }
        Some(decl_id) => {
            working_set.enter_scope();
            let ParsedInternalCall { call, output } =
                parse_internal_call(working_set, spans[0], &spans[1..], decl_id);

            working_set.exit_scope();

            let call_span = span(spans);
            let decl = working_set.get_decl(decl_id);
            let sig = decl.signature();

            let starting_error_count = working_set.parse_errors.len();
            check_call(working_set, call_span, &sig, &call);

            let Ok(is_help) = has_flag_const(working_set, &call, "help") else {
                return garbage(spans[0]);
            };

            if starting_error_count != working_set.parse_errors.len() || is_help {
                return Expression {
                    expr: Expr::Call(call),
                    span: call_span,
                    ty: output,
                    custom_completion: None,
                };
            }

            // Let's get our block and make sure it has the right signature
            if let Some(arg) = call.positional_nth(2) {
                match arg {
                    Expression {
                        expr: Expr::Block(block_id),
                        ..
                    }
                    | Expression {
                        expr: Expr::RowCondition(block_id),
                        ..
                    } => {
                        let block = working_set.get_block_mut(*block_id);

                        block.signature = Box::new(sig);
                    }
                    _ => {}
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
            },
        );
    }

    Expression {
        expr: Expr::Call(call),
        span: call_span,
        ty: Type::Nothing,
        custom_completion: None,
    }
}

/// If `name` is a keyword, emit an error.
fn verify_not_reserved_variable_name(working_set: &mut StateWorkingSet, name: &str, span: Span) {
    if RESERVED_VARIABLE_NAMES.contains(&name) {
        working_set.error(ParseError::NameIsBuiltinVar(name.to_string(), span))
    }
}

// Returns also the parsed command name and ID
pub fn parse_def(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
    module_name: Option<&[u8]>,
) -> (Pipeline, Option<(Vec<u8>, DeclId)>) {
    let spans = &lite_command.parts[..];

    let (usage, extra_usage) = working_set.build_usage(&lite_command.comments);

    // Checking that the function is used with the correct name
    // Maybe this is not necessary but it is a sanity check
    // Note: "export def" is treated the same as "def"

    let (name_span, split_id) =
        if spans.len() > 1 && working_set.get_span_contents(spans[0]) == b"export" {
            (spans[1], 2)
        } else {
            (spans[0], 1)
        };

    let def_call = working_set.get_span_contents(name_span).to_vec();
    if def_call != b"def" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for def function".into(),
            span(spans),
        ));
        return (garbage_pipeline(spans), None);
    }
    if let Some(redirection) = lite_command.redirection.as_ref() {
        working_set.error(redirecting_builtin_error("def", redirection));
        return (garbage_pipeline(spans), None);
    }

    // Parsing the spans and checking that they match the register signature
    // Using a parsed call makes more sense than checking for how many spans are in the call
    // Also, by creating a call, it can be checked if it matches the declaration signature
    let (call, call_span) = match working_set.find_decl(&def_call) {
        None => {
            working_set.error(ParseError::UnknownState(
                "internal error: def declaration not found".into(),
                span(spans),
            ));
            return (garbage_pipeline(spans), None);
        }
        Some(decl_id) => {
            working_set.enter_scope();
            let (command_spans, rest_spans) = spans.split_at(split_id);

            // Find the first span that is not a flag
            let mut decl_name_span = None;

            for span in rest_spans {
                if !working_set.get_span_contents(*span).starts_with(&[b'-']) {
                    decl_name_span = Some(*span);
                    break;
                }
            }

            if let Some(name_span) = decl_name_span {
                // Check whether name contains [] or () -- possible missing space error
                if let Some(err) = detect_params_in_name(
                    working_set,
                    name_span,
                    String::from_utf8_lossy(&def_call).as_ref(),
                ) {
                    working_set.error(err);
                    return (garbage_pipeline(spans), None);
                }
            }

            let starting_error_count = working_set.parse_errors.len();
            let ParsedInternalCall { call, output } =
                parse_internal_call(working_set, span(command_spans), rest_spans, decl_id);
            // This is to preserve the order of the errors so that
            // the check errors below come first
            let mut new_errors = working_set.parse_errors[starting_error_count..].to_vec();
            working_set.parse_errors.truncate(starting_error_count);

            working_set.exit_scope();

            let call_span = span(spans);
            let decl = working_set.get_decl(decl_id);
            let sig = decl.signature();

            // Let's get our block and make sure it has the right signature
            if let Some(arg) = call.positional_nth(2) {
                match arg {
                    Expression {
                        expr: Expr::Closure(block_id),
                        ..
                    } => {
                        let block = working_set.get_block_mut(*block_id);
                        block.signature = Box::new(sig.clone());
                    }
                    _ => working_set.error(ParseError::Expected(
                        "definition body closure { ... }",
                        arg.span,
                    )),
                }
            }

            let starting_error_count = working_set.parse_errors.len();
            check_call(working_set, call_span, &sig, &call);
            working_set.parse_errors.append(&mut new_errors);

            let Ok(is_help) = has_flag_const(working_set, &call, "help") else {
                return (garbage_pipeline(spans), None);
            };

            if starting_error_count != working_set.parse_errors.len() || is_help {
                return (
                    Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: call_span,
                        ty: output,
                        custom_completion: None,
                    }]),
                    None,
                );
            }

            (call, call_span)
        }
    };

    let Ok(has_env) = has_flag_const(working_set, &call, "env") else {
        return (garbage_pipeline(spans), None);
    };
    let Ok(has_wrapped) = has_flag_const(working_set, &call, "wrapped") else {
        return (garbage_pipeline(spans), None);
    };

    // All positional arguments must be in the call positional vector by this point
    let name_expr = call.positional_nth(0).expect("def call already checked");
    let sig = call.positional_nth(1).expect("def call already checked");
    let block = call.positional_nth(2).expect("def call already checked");

    let name = if let Some(name) = name_expr.as_string() {
        if let Some(mod_name) = module_name {
            if name.as_bytes() == mod_name {
                let name_expr_span = name_expr.span;

                working_set.error(ParseError::NamedAsModule(
                    "command".to_string(),
                    name,
                    "main".to_string(),
                    name_expr_span,
                ));
                return (
                    Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: call_span,
                        ty: Type::Any,
                        custom_completion: None,
                    }]),
                    None,
                );
            }
        }

        name
    } else {
        working_set.error(ParseError::UnknownState(
            "Could not get string from string expression".into(),
            name_expr.span,
        ));
        return (garbage_pipeline(spans), None);
    };

    let mut result = None;

    if let (Some(mut signature), Some(block_id)) = (sig.as_signature(), block.as_block()) {
        for arg_name in &signature.required_positional {
            verify_not_reserved_variable_name(working_set, &arg_name.name, sig.span);
        }
        for arg_name in &signature.optional_positional {
            verify_not_reserved_variable_name(working_set, &arg_name.name, sig.span);
        }
        for arg_name in &signature.rest_positional {
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
                            Pipeline::from_vec(vec![Expression {
                                expr: Expr::Call(call),
                                span: call_span,
                                ty: Type::Any,
                                custom_completion: None,
                            }]),
                            result,
                        );
                    }
                }
            } else {
                working_set.error(ParseError::MissingPositional("...rest-like positional argument".to_string(), name_expr.span, "def --wrapped must have a ...rest-like positional argument. Add '...rest: string' to the command's signature.".to_string()));

                return (
                    Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: call_span,
                        ty: Type::Any,
                        custom_completion: None,
                    }]),
                    result,
                );
            }
        }

        if let Some(decl_id) = working_set.find_predecl(name.as_bytes()) {
            let declaration = working_set.get_decl_mut(decl_id);

            signature.name.clone_from(&name);
            if !has_wrapped {
                *signature = signature.add_help();
            }
            signature.usage = usage;
            signature.extra_usage = extra_usage;
            signature.allows_unknown_args = has_wrapped;

            *declaration = signature.clone().into_block_command(block_id);

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
        Pipeline::from_vec(vec![Expression {
            expr: Expr::Call(call),
            span: call_span,
            ty: Type::Any,
            custom_completion: None,
        }]),
        result,
    )
}

pub fn parse_extern(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
    module_name: Option<&[u8]>,
) -> Pipeline {
    let spans = &lite_command.parts;

    let (usage, extra_usage) = working_set.build_usage(&lite_command.comments);

    // Checking that the function is used with the correct name
    // Maybe this is not necessary but it is a sanity check

    let (name_span, split_id) =
        if spans.len() > 1 && (working_set.get_span_contents(spans[0]) == b"export") {
            (spans[1], 2)
        } else {
            (spans[0], 1)
        };

    let extern_call = working_set.get_span_contents(name_span).to_vec();
    if extern_call != b"extern" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for extern command".into(),
            span(spans),
        ));
        return garbage_pipeline(spans);
    }
    if let Some(redirection) = lite_command.redirection.as_ref() {
        working_set.error(redirecting_builtin_error("extern", redirection));
        return garbage_pipeline(spans);
    }

    // Parsing the spans and checking that they match the register signature
    // Using a parsed call makes more sense than checking for how many spans are in the call
    // Also, by creating a call, it can be checked if it matches the declaration signature
    let (call, call_span) = match working_set.find_decl(&extern_call) {
        None => {
            working_set.error(ParseError::UnknownState(
                "internal error: def declaration not found".into(),
                span(spans),
            ));
            return garbage_pipeline(spans);
        }
        Some(decl_id) => {
            working_set.enter_scope();

            let (command_spans, rest_spans) = spans.split_at(split_id);

            if let Some(name_span) = rest_spans.first() {
                if let Some(err) = detect_params_in_name(
                    working_set,
                    *name_span,
                    String::from_utf8_lossy(&extern_call).as_ref(),
                ) {
                    working_set.error(err);
                    return garbage_pipeline(spans);
                }
            }

            let ParsedInternalCall { call, .. } =
                parse_internal_call(working_set, span(command_spans), rest_spans, decl_id);
            working_set.exit_scope();

            let call_span = span(spans);
            //let decl = working_set.get_decl(decl_id);
            //let sig = decl.signature();

            (call, call_span)
        }
    };
    let name_expr = call.positional_nth(0);
    let sig = call.positional_nth(1);
    let body = call.positional_nth(2);

    if let (Some(name_expr), Some(sig)) = (name_expr, sig) {
        if let (Some(name), Some(mut signature)) = (&name_expr.as_string(), sig.as_signature()) {
            if let Some(mod_name) = module_name {
                if name.as_bytes() == mod_name {
                    let name_expr_span = name_expr.span;
                    working_set.error(ParseError::NamedAsModule(
                        "known external".to_string(),
                        name.clone(),
                        "main".to_string(),
                        name_expr_span,
                    ));
                    return Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: call_span,
                        ty: Type::Any,
                        custom_completion: None,
                    }]);
                }
            }

            if let Some(decl_id) = working_set.find_predecl(name.as_bytes()) {
                let declaration = working_set.get_decl_mut(decl_id);

                let external_name = if let Some(mod_name) = module_name {
                    if name.as_bytes() == b"main" {
                        String::from_utf8_lossy(mod_name).to_string()
                    } else {
                        name.clone()
                    }
                } else {
                    name.clone()
                };

                signature.name.clone_from(&external_name);
                signature.usage.clone_from(&usage);
                signature.extra_usage.clone_from(&extra_usage);
                signature.allows_unknown_args = true;

                if let Some(block_id) = body.and_then(|x| x.as_block()) {
                    if signature.rest_positional.is_none() {
                        working_set.error(ParseError::InternalError(
                            "Extern block must have a rest positional argument".into(),
                            name_expr.span,
                        ));
                    } else {
                        *declaration = signature.clone().into_block_command(block_id);

                        working_set.get_block_mut(block_id).signature = signature;
                    }
                } else {
                    let decl = KnownExternal {
                        name: external_name,
                        usage,
                        extra_usage,
                        signature,
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

    Pipeline::from_vec(vec![Expression {
        expr: Expr::Call(call),
        span: call_span,
        ty: Type::Any,
        custom_completion: None,
    }])
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

    if spans.len() == 1 {
        None
    } else if spans.len() < command_len + 3 {
        if working_set.get_span_contents(spans[command_len]) == b"=" {
            let name =
                String::from_utf8_lossy(working_set.get_span_contents(span(&spans[..command_len])));
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
        let name =
            String::from_utf8_lossy(working_set.get_span_contents(span(&spans[..command_len])));
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
            span(spans),
        ));
        return garbage_pipeline(spans);
    }
    if let Some(redirection) = lite_command.redirection.as_ref() {
        working_set.error(redirecting_builtin_error("alias", redirection));
        return garbage_pipeline(spans);
    }

    if let Some(span) = check_alias_name(working_set, spans) {
        return Pipeline::from_vec(vec![garbage(*span)]);
    }

    if let Some(decl_id) = working_set.find_decl(b"alias") {
        let (command_spans, rest_spans) = spans.split_at(split_id);

        let original_starting_error_count = working_set.parse_errors.len();

        let ParsedInternalCall {
            call: alias_call,
            output,
            ..
        } = parse_internal_call(working_set, span(command_spans), rest_spans, decl_id);

        working_set
            .parse_errors
            .truncate(original_starting_error_count);

        let Ok(has_help_flag) = has_flag_const(working_set, &alias_call, "help") else {
            return garbage_pipeline(spans);
        };

        let alias_pipeline = Pipeline::from_vec(vec![Expression {
            expr: Expr::Call(alias_call.clone()),
            span: span(spans),
            ty: output,
            custom_completion: None,
        }]);

        if has_help_flag {
            return alias_pipeline;
        }

        let Some(alias_name_expr) = alias_call.positional_nth(0) else {
            working_set.error(ParseError::UnknownState(
                "Missing positional after call check".to_string(),
                span(spans),
            ));
            return garbage_pipeline(spans);
        };

        let alias_name = if let Some(name) = alias_name_expr.as_string() {
            if name.contains('#')
                || name.contains('^')
                || name.parse::<bytesize::ByteSize>().is_ok()
                || name.parse::<f64>().is_ok()
            {
                working_set.error(ParseError::AliasNotValid(alias_name_expr.span));
                return garbage_pipeline(spans);
            } else {
                name
            }
        } else {
            working_set.error(ParseError::AliasNotValid(alias_name_expr.span));
            return garbage_pipeline(spans);
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

            if starting_error_count != working_set.parse_errors.len() {
                if let Some(e) = working_set.parse_errors.get(starting_error_count) {
                    if let ParseError::MissingPositional(..) = e {
                        working_set
                            .parse_errors
                            .truncate(original_starting_error_count);
                        // ignore missing required positional
                    } else {
                        return garbage_pipeline(replacement_spans);
                    }
                }
            }

            let (command, wrapped_call) = match expr {
                Expression {
                    expr: Expr::Call(ref rhs_call),
                    ..
                } => {
                    let cmd = working_set.get_decl(rhs_call.decl_id);

                    if cmd.is_parser_keyword()
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

            // Tries to build a useful usage string
            let (usage, extra_usage) = match lite_command.comments.is_empty() {
                // First from comments, if any are present
                false => working_set.build_usage(&lite_command.comments),
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
                usage,
                extra_usage,
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
                span(&spans[..split_id]),
                "incomplete alias".into(),
            ));
        }

        return alias_pipeline;
    }

    working_set.error(ParseError::InternalError(
        "Alias statement unparsable".into(),
        span(spans),
    ));

    garbage_pipeline(spans)
}

// This one will trigger if `export` appears during eval, e.g., in a script
pub fn parse_export_in_block(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
) -> Pipeline {
    let call_span = span(&lite_command.parts);

    let full_name = if lite_command.parts.len() > 1 {
        let sub = working_set.get_span_contents(lite_command.parts[1]);
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
        return garbage_pipeline(&lite_command.parts);
    }

    if let Some(decl_id) = working_set.find_decl(full_name.as_bytes()) {
        let ParsedInternalCall { call, output, .. } = parse_internal_call(
            working_set,
            if full_name == "export" {
                lite_command.parts[0]
            } else {
                span(&lite_command.parts[0..2])
            },
            if full_name == "export" {
                &lite_command.parts[1..]
            } else {
                &lite_command.parts[2..]
            },
            decl_id,
        );

        let decl = working_set.get_decl(decl_id);

        let starting_error_count = working_set.parse_errors.len();
        check_call(working_set, call_span, &decl.signature(), &call);

        let Ok(is_help) = has_flag_const(working_set, &call, "help") else {
            return garbage_pipeline(&lite_command.parts);
        };

        if starting_error_count != working_set.parse_errors.len() || is_help {
            return Pipeline::from_vec(vec![Expression {
                expr: Expr::Call(call),
                span: call_span,
                ty: output,
                custom_completion: None,
            }]);
        }
    } else {
        working_set.error(ParseError::UnknownState(
            format!("internal error: '{full_name}' declaration not found",),
            span(&lite_command.parts),
        ));
        return garbage_pipeline(&lite_command.parts);
    };

    if full_name == "export" {
        // export by itself is meaningless
        working_set.error(ParseError::UnexpectedKeyword(
            "export".into(),
            lite_command.parts[0],
        ));
        return garbage_pipeline(&lite_command.parts);
    }

    match full_name {
        "export alias" => parse_alias(working_set, lite_command, None),
        "export def" => parse_def(working_set, lite_command, None).0,
        "export const" => parse_const(working_set, &lite_command.parts[1..]),
        "export use" => parse_use(working_set, lite_command).0,
        "export module" => parse_module(working_set, lite_command, None).0,
        "export extern" => parse_extern(working_set, lite_command, None),
        _ => {
            working_set.error(ParseError::UnexpectedKeyword(
                full_name.into(),
                lite_command.parts[0],
            ));

            garbage_pipeline(&lite_command.parts)
        }
    }
}

// This one will trigger only in a module
pub fn parse_export_in_module(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
    module_name: &[u8],
) -> (Pipeline, Vec<Exportable>) {
    let spans = &lite_command.parts[..];

    let export_span = if let Some(sp) = spans.first() {
        if working_set.get_span_contents(*sp) != b"export" {
            working_set.error(ParseError::UnknownState(
                "expected export statement".into(),
                span(spans),
            ));
            return (garbage_pipeline(spans), vec![]);
        }

        *sp
    } else {
        working_set.error(ParseError::UnknownState(
            "got empty input for parsing export statement".into(),
            span(spans),
        ));
        return (garbage_pipeline(spans), vec![]);
    };

    let export_decl_id = if let Some(id) = working_set.find_decl(b"export") {
        id
    } else {
        working_set.error(ParseError::InternalError(
            "missing export command".into(),
            export_span,
        ));
        return (garbage_pipeline(spans), vec![]);
    };

    let mut call = Box::new(Call {
        head: spans[0],
        decl_id: export_decl_id,
        arguments: vec![],
        parser_info: HashMap::new(),
    });

    let exportables = if let Some(kw_span) = spans.get(1) {
        let kw_name = working_set.get_span_contents(*kw_span);
        match kw_name {
            b"def" => {
                let lite_command = LiteCommand {
                    comments: lite_command.comments.clone(),
                    parts: spans[1..].to_vec(),
                    pipe: lite_command.pipe,
                    redirection: lite_command.redirection.clone(),
                };
                let (pipeline, cmd_result) =
                    parse_def(working_set, &lite_command, Some(module_name));

                let mut result = vec![];

                if let Some((decl_name, decl_id)) = cmd_result {
                    result.push(Exportable::Decl {
                        name: decl_name.to_vec(),
                        id: decl_id,
                    });
                }

                let export_def_decl_id = if let Some(id) = working_set.find_decl(b"export def") {
                    id
                } else {
                    working_set.error(ParseError::InternalError(
                        "missing 'export def' command".into(),
                        export_span,
                    ));
                    return (garbage_pipeline(spans), vec![]);
                };

                // Trying to warp the 'def' call into the 'export def' in a very clumsy way
                if let Some(Expr::Call(def_call)) = pipeline.elements.first().map(|e| &e.expr.expr)
                {
                    call.clone_from(def_call);
                    call.head = span(&spans[0..=1]);
                    call.decl_id = export_def_decl_id;
                } else {
                    working_set.error(ParseError::InternalError(
                        "unexpected output from parsing a definition".into(),
                        span(&spans[1..]),
                    ));
                };

                result
            }
            b"extern" => {
                let lite_command = LiteCommand {
                    comments: lite_command.comments.clone(),
                    parts: spans[1..].to_vec(),
                    pipe: lite_command.pipe,
                    redirection: lite_command.redirection.clone(),
                };
                let extern_name = [b"export ", kw_name].concat();

                let pipeline = parse_extern(working_set, &lite_command, Some(module_name));

                let export_def_decl_id = if let Some(id) = working_set.find_decl(&extern_name) {
                    id
                } else {
                    working_set.error(ParseError::InternalError(
                        "missing 'export extern' command".into(),
                        export_span,
                    ));
                    return (garbage_pipeline(spans), vec![]);
                };

                // Trying to warp the 'def' call into the 'export def' in a very clumsy way
                if let Some(Expr::Call(def_call)) = pipeline.elements.first().map(|e| &e.expr.expr)
                {
                    call.clone_from(def_call);
                    call.head = span(&spans[0..=1]);
                    call.decl_id = export_def_decl_id;
                } else {
                    working_set.error(ParseError::InternalError(
                        "unexpected output from parsing a definition".into(),
                        span(&spans[1..]),
                    ));
                };

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
                        span(&spans[1..]),
                    ));
                }

                result
            }
            b"alias" => {
                let lite_command = LiteCommand {
                    comments: lite_command.comments.clone(),
                    parts: spans[1..].to_vec(),
                    pipe: lite_command.pipe,
                    redirection: lite_command.redirection.clone(),
                };
                let pipeline = parse_alias(working_set, &lite_command, Some(module_name));

                let export_alias_decl_id = if let Some(id) = working_set.find_decl(b"export alias")
                {
                    id
                } else {
                    working_set.error(ParseError::InternalError(
                        "missing 'export alias' command".into(),
                        export_span,
                    ));
                    return (garbage_pipeline(spans), vec![]);
                };

                // Trying to warp the 'alias' call into the 'export alias' in a very clumsy way
                if let Some(Expr::Call(alias_call)) =
                    pipeline.elements.first().map(|e| &e.expr.expr)
                {
                    call.clone_from(alias_call);

                    call.head = span(&spans[0..=1]);
                    call.decl_id = export_alias_decl_id;
                } else {
                    working_set.error(ParseError::InternalError(
                        "unexpected output from parsing a definition".into(),
                        span(&spans[1..]),
                    ));
                };

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
                        span(&spans[1..]),
                    ));
                }

                result
            }
            b"use" => {
                let lite_command = LiteCommand {
                    comments: lite_command.comments.clone(),
                    parts: spans[1..].to_vec(),
                    pipe: lite_command.pipe,
                    redirection: lite_command.redirection.clone(),
                };
                let (pipeline, exportables) = parse_use(working_set, &lite_command);

                let export_use_decl_id = if let Some(id) = working_set.find_decl(b"export use") {
                    id
                } else {
                    working_set.error(ParseError::InternalError(
                        "missing 'export use' command".into(),
                        export_span,
                    ));
                    return (garbage_pipeline(spans), vec![]);
                };

                // Trying to warp the 'use' call into the 'export use' in a very clumsy way
                if let Some(Expr::Call(use_call)) = pipeline.elements.first().map(|e| &e.expr.expr)
                {
                    call.clone_from(use_call);

                    call.head = span(&spans[0..=1]);
                    call.decl_id = export_use_decl_id;
                } else {
                    working_set.error(ParseError::InternalError(
                        "unexpected output from parsing a definition".into(),
                        span(&spans[1..]),
                    ));
                };

                exportables
            }
            b"module" => {
                let (pipeline, maybe_module_id) =
                    parse_module(working_set, lite_command, Some(module_name));

                let export_module_decl_id =
                    if let Some(id) = working_set.find_decl(b"export module") {
                        id
                    } else {
                        working_set.error(ParseError::InternalError(
                            "missing 'export module' command".into(),
                            export_span,
                        ));
                        return (garbage_pipeline(spans), vec![]);
                    };

                // Trying to warp the 'module' call into the 'export module' in a very clumsy way
                if let Some(Expr::Call(module_call)) =
                    pipeline.elements.first().map(|e| &e.expr.expr)
                {
                    call.clone_from(module_call);

                    call.head = span(&spans[0..=1]);
                    call.decl_id = export_module_decl_id;
                } else {
                    working_set.error(ParseError::InternalError(
                        "unexpected output from parsing a definition".into(),
                        span(&spans[1..]),
                    ));
                };

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
                            span(&spans[1..]),
                        ));
                    }
                }

                result
            }
            b"const" => {
                let pipeline = parse_const(working_set, &spans[1..]);
                let export_const_decl_id = if let Some(id) = working_set.find_decl(b"export const")
                {
                    id
                } else {
                    working_set.error(ParseError::InternalError(
                        "missing 'export const' command".into(),
                        export_span,
                    ));
                    return (garbage_pipeline(spans), vec![]);
                };

                // Trying to warp the 'const' call into the 'export const' in a very clumsy way
                if let Some(Expr::Call(def_call)) = pipeline.elements.first().map(|e| &e.expr.expr)
                {
                    call.clone_from(def_call);

                    call.head = span(&spans[0..=1]);
                    call.decl_id = export_const_decl_id;
                } else {
                    working_set.error(ParseError::InternalError(
                        "unexpected output from parsing a definition".into(),
                        span(&spans[1..]),
                    ));
                };

                let mut result = vec![];

                if let Some(var_name_span) = spans.get(2) {
                    let var_name = working_set.get_span_contents(*var_name_span);
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
                            span(&spans[1..]),
                        ));
                    }
                }

                result
            }
            _ => {
                working_set.error(ParseError::Expected(
                    "def, alias, use, module, const or extern keyword",
                    spans[1],
                ));

                vec![]
            }
        }
    } else {
        working_set.error(ParseError::MissingPositional(
            "def, alias, use, module, const or extern keyword".to_string(),
            Span::new(export_span.end, export_span.end),
            "def, alias, use, module, const or extern keyword".to_string(),
        ));

        vec![]
    };

    (
        Pipeline::from_vec(vec![Expression {
            expr: Expr::Call(call),
            span: span(spans),
            ty: Type::Any,
            custom_completion: None,
        }]),
        exportables,
    )
}

pub fn parse_export_env(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
) -> (Pipeline, Option<BlockId>) {
    if !spans.is_empty() && working_set.get_span_contents(spans[0]) != b"export-env" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for 'export-env' command".into(),
            span(spans),
        ));
        return (garbage_pipeline(spans), None);
    }

    if spans.len() < 2 {
        working_set.error(ParseError::MissingPositional(
            "block".into(),
            span(spans),
            "export-env <block>".into(),
        ));
        return (garbage_pipeline(spans), None);
    }

    let call = match working_set.find_decl(b"export-env") {
        Some(decl_id) => {
            let ParsedInternalCall { call, output } =
                parse_internal_call(working_set, spans[0], &[spans[1]], decl_id);
            let decl = working_set.get_decl(decl_id);

            let call_span = span(spans);

            let starting_error_count = working_set.parse_errors.len();
            check_call(working_set, call_span, &decl.signature(), &call);

            let Ok(is_help) = has_flag_const(working_set, &call, "help") else {
                return (garbage_pipeline(spans), None);
            };

            if starting_error_count != working_set.parse_errors.len() || is_help {
                return (
                    Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: call_span,
                        ty: output,
                        custom_completion: None,
                    }]),
                    None,
                );
            }

            call
        }
        None => {
            working_set.error(ParseError::UnknownState(
                "internal error: 'export-env' declaration not found".into(),
                span(spans),
            ));
            return (garbage_pipeline(spans), None);
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
            return (garbage_pipeline(spans), None);
        }
    } else {
        working_set.error(ParseError::UnknownState(
            "internal error: 'export-env' block is missing".into(),
            span(spans),
        ));
        return (garbage_pipeline(spans), None);
    };

    let pipeline = Pipeline::from_vec(vec![Expression {
        expr: Expr::Call(call),
        span: span(spans),
        ty: Type::Any,
        custom_completion: None,
    }]);

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
                {
                    if !comments.is_empty() {
                        break;
                    }
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

    let (output, err) = lite_parse(&output);
    if let Some(err) = err {
        working_set.error(err)
    }

    for pipeline in &output.block {
        if pipeline.commands.len() == 1 {
            parse_def_predecl(working_set, &pipeline.commands[0].parts);
        }
    }

    let mut module = Module::from_span(module_name.to_vec(), span);

    let mut block = Block::new_with_capacity(output.block.len());

    for pipeline in output.block.iter() {
        if pipeline.commands.len() == 1 {
            let command = &pipeline.commands[0];

            let name = working_set.get_span_contents(command.parts[0]);

            match name {
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
                b"const" => block
                    .pipelines
                    .push(parse_const(working_set, &command.parts)),
                b"extern" => block
                    .pipelines
                    .push(parse_extern(working_set, command, None)),
                b"alias" => {
                    block.pipelines.push(parse_alias(
                        working_set,
                        command,
                        None, // using aliases named as the module locally is OK
                    ))
                }
                b"use" => {
                    let (pipeline, _) = parse_use(working_set, command);

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
                b"export" => {
                    let (pipe, exportables) =
                        parse_export_in_module(working_set, command, module_name);

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

                    block.pipelines.push(garbage_pipeline(&command.parts))
                }
            }
        } else {
            working_set.error(ParseError::Expected("not a pipeline", span));
            block.pipelines.push(garbage_pipeline(&[span]))
        }
    }

    working_set.exit_scope();

    (block, module, module_comments)
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
    if let Some(module_id) = working_set.find_module_by_span(new_span) {
        return Some(module_id);
    }

    // Add the file to the stack of files being processed.
    if let Err(e) = working_set.files.push(path.path_buf(), path_span) {
        working_set.error(e);
        return None;
    }

    // Parse the module
    let (block, module, module_comments) =
        parse_module_block(working_set, new_span, module_name.as_bytes());

    // Remove the file from the stack of files being processed.
    working_set.files.pop();

    let _ = working_set.add_block(Arc::new(block));
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
        return (garbage_pipeline(spans), None);
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

            let ParsedInternalCall { call, output } =
                parse_internal_call(working_set, span(command_spans), rest_spans, decl_id);
            let decl = working_set.get_decl(decl_id);

            let call_span = span(spans);

            let starting_error_count = working_set.parse_errors.len();
            check_call(working_set, call_span, &decl.signature(), &call);

            let Ok(is_help) = has_flag_const(working_set, &call, "help") else {
                return (garbage_pipeline(spans), None);
            };

            if starting_error_count != working_set.parse_errors.len() || is_help {
                return (
                    Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: call_span,
                        ty: output,
                        custom_completion: None,
                    }]),
                    None,
                );
            }

            (call, call_span)
        }
        None => {
            working_set.error(ParseError::UnknownState(
                "internal error: 'module' or 'export module' declaration not found".into(),
                span(spans),
            ));
            return (garbage_pipeline(spans), None);
        }
    };

    let (module_name_or_path, module_name_or_path_span, module_name_or_path_expr) =
        if let Some(name) = call.positional_nth(0) {
            if let Some(s) = name.as_string() {
                if let Some(mod_name) = module_name {
                    if s.as_bytes() == mod_name {
                        working_set.error(ParseError::NamedAsModule(
                            "module".to_string(),
                            s,
                            "mod".to_string(),
                            name.span,
                        ));
                        return (
                            Pipeline::from_vec(vec![Expression {
                                expr: Expr::Call(call),
                                span: call_span,
                                ty: Type::Any,
                                custom_completion: None,
                            }]),
                            None,
                        );
                    }
                }
                (s, name.span, name.clone())
            } else {
                working_set.error(ParseError::UnknownState(
                    "internal error: name not a string".into(),
                    span(spans),
                ));
                return (garbage_pipeline(spans), None);
            }
        } else {
            working_set.error(ParseError::UnknownState(
                "internal error: missing positional".into(),
                span(spans),
            ));
            return (garbage_pipeline(spans), None);
        };

    let pipeline = Pipeline::from_vec(vec![Expression {
        expr: Expr::Call(call),
        span: call_span,
        ty: Type::Any,
        custom_completion: None,
    }]);

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
            span(spans),
        ));

        return (garbage_pipeline(spans), None);
    }

    let module_name = module_name_or_path;

    let block_span = spans[split_id + 1];
    let block_bytes = working_set.get_span_contents(block_span);
    let mut start = block_span.start;
    let mut end = block_span.end;

    if block_bytes.starts_with(b"{") {
        start += 1;
    } else {
        working_set.error(ParseError::Expected("block", block_span));
        return (garbage_pipeline(spans), None);
    }

    if block_bytes.ends_with(b"}") {
        end -= 1;
    } else {
        working_set.error(ParseError::Unclosed("}".into(), Span::new(end, end)));
    }

    let block_span = Span::new(start, end);

    let (block, module, inner_comments) =
        parse_module_block(working_set, block_span, module_name.as_bytes());

    let block_id = working_set.add_block(Arc::new(block));

    module_comments.extend(inner_comments);
    let module_id = working_set.add_module(&module_name, module, module_comments);

    let block_expr = Expression {
        expr: Expr::Block(block_id),
        span: block_span,
        ty: Type::Block,
        custom_completion: None,
    };

    let module_decl_id = working_set
        .find_decl(b"module")
        .expect("internal error: missing module command");

    let call = Box::new(Call {
        head: span(&spans[..split_id]),
        decl_id: module_decl_id,
        arguments: vec![
            Argument::Positional(module_name_or_path_expr),
            Argument::Positional(block_expr),
        ],
        parser_info: HashMap::new(),
    });

    (
        Pipeline::from_vec(vec![Expression {
            expr: Expr::Call(call),
            span: span(spans),
            ty: Type::Any,
            custom_completion: None,
        }]),
        Some(module_id),
    )
}

pub fn parse_use(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
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
            span(spans),
        ));
        return (garbage_pipeline(spans), vec![]);
    }

    if working_set.get_span_contents(name_span) != b"use" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for 'use' command".into(),
            span(spans),
        ));
        return (garbage_pipeline(spans), vec![]);
    }

    if let Some(redirection) = lite_command.redirection.as_ref() {
        working_set.error(redirecting_builtin_error("use", redirection));
        return (garbage_pipeline(spans), vec![]);
    }

    let (call, call_span, args_spans) = match working_set.find_decl(b"use") {
        Some(decl_id) => {
            let (command_spans, rest_spans) = spans.split_at(split_id);

            let ParsedInternalCall { call, output } =
                parse_internal_call(working_set, span(command_spans), rest_spans, decl_id);
            let decl = working_set.get_decl(decl_id);

            let call_span = span(spans);

            let starting_error_count = working_set.parse_errors.len();
            check_call(working_set, call_span, &decl.signature(), &call);

            let Ok(is_help) = has_flag_const(working_set, &call, "help") else {
                return (garbage_pipeline(spans), vec![]);
            };

            if starting_error_count != working_set.parse_errors.len() || is_help {
                return (
                    Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: call_span,
                        ty: output,
                        custom_completion: None,
                    }]),
                    vec![],
                );
            }

            (call, call_span, rest_spans)
        }
        None => {
            working_set.error(ParseError::UnknownState(
                "internal error: 'use' declaration not found".into(),
                span(spans),
            ));
            return (garbage_pipeline(spans), vec![]);
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
        return (garbage_pipeline(spans), vec![]);
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
            Pipeline::from_vec(vec![Expression {
                expr: Expr::Call(call),
                span: call_span,
                ty: Type::Any,
                custom_completion: None,
            }]),
            vec![],
        );
    };

    let (definitions, errors) = module.resolve_import_pattern(
        working_set,
        module_id,
        &import_pattern.members,
        None,
        name_span,
    );

    working_set.parse_errors.extend(errors);

    let mut constants = vec![];

    for (name, const_val) in definitions.constants {
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

    // Extend the current scope with the module's exportables
    working_set.use_decls(definitions.decls);
    working_set.use_modules(definitions.modules);
    working_set.use_variables(constants);

    // Create a new Use command call to pass the import pattern as parser info
    let import_pattern_expr = Expression {
        expr: Expr::ImportPattern(Box::new(import_pattern)),
        span: span(args_spans),
        ty: Type::Any,
        custom_completion: None,
    };

    let mut call = call;
    call.set_parser_info("import_pattern".to_string(), import_pattern_expr);

    (
        Pipeline::from_vec(vec![Expression {
            expr: Expr::Call(call),
            span: span(spans),
            ty: Type::Any,
            custom_completion: None,
        }]),
        exportables,
    )
}

pub fn parse_hide(working_set: &mut StateWorkingSet, lite_command: &LiteCommand) -> Pipeline {
    let spans = &lite_command.parts;

    if working_set.get_span_contents(spans[0]) != b"hide" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for 'hide' command".into(),
            span(spans),
        ));
        return garbage_pipeline(spans);
    }
    if let Some(redirection) = lite_command.redirection.as_ref() {
        working_set.error(redirecting_builtin_error("hide", redirection));
        return garbage_pipeline(spans);
    }

    let (call, args_spans) = match working_set.find_decl(b"hide") {
        Some(decl_id) => {
            let ParsedInternalCall { call, output } =
                parse_internal_call(working_set, spans[0], &spans[1..], decl_id);
            let decl = working_set.get_decl(decl_id);

            let call_span = span(spans);

            let starting_error_count = working_set.parse_errors.len();
            check_call(working_set, call_span, &decl.signature(), &call);

            let Ok(is_help) = has_flag_const(working_set, &call, "help") else {
                return garbage_pipeline(spans);
            };

            if starting_error_count != working_set.parse_errors.len() || is_help {
                return Pipeline::from_vec(vec![Expression {
                    expr: Expr::Call(call),
                    span: call_span,
                    ty: output,
                    custom_completion: None,
                }]);
            }

            (call, &spans[1..])
        }
        None => {
            working_set.error(ParseError::UnknownState(
                "internal error: 'hide' declaration not found".into(),
                span(spans),
            ));
            return garbage_pipeline(spans);
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
        return garbage_pipeline(spans);
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
                return garbage_pipeline(spans);
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
        let import_pattern_expr = Expression {
            expr: Expr::ImportPattern(Box::new(import_pattern)),
            span: span(args_spans),
            ty: Type::Any,
            custom_completion: None,
        };

        let mut call = call;
        call.set_parser_info("import_pattern".to_string(), import_pattern_expr);

        Pipeline::from_vec(vec![Expression {
            expr: Expr::Call(call),
            span: span(spans),
            ty: Type::Any,
            custom_completion: None,
        }])
    } else {
        working_set.error(ParseError::UnknownState(
            "Expected structure: hide <name>".into(),
            span(spans),
        ));
        garbage_pipeline(spans)
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
                    return garbage_pipeline(&[call_span]);
                }
            },
            Err(err) => {
                working_set.error(err.wrap(working_set, call_span));
                return garbage_pipeline(&[call_span]);
            }
        }
    } else {
        working_set.error(ParseError::UnknownState(
            "internal error: Missing required positional after call parsing".into(),
            call_span,
        ));
        return garbage_pipeline(&[call_span]);
    };

    let pipeline = Pipeline::from_vec(vec![Expression {
        expr: Expr::Call(call),
        span: call_span,
        ty: Type::Any,
        custom_completion: None,
    }]);

    let module_id = working_set.add_module(
        &overlay_name,
        Module::new(overlay_name.as_bytes().to_vec()),
        vec![],
    );

    working_set.add_overlay(
        overlay_name.as_bytes().to_vec(),
        module_id,
        vec![],
        vec![],
        false,
    );

    pipeline
}

pub fn parse_overlay_use(working_set: &mut StateWorkingSet, call: Box<Call>) -> Pipeline {
    let call_span = call.span();

    let (overlay_name, overlay_name_span) = if let Some(expr) = call.positional_nth(0) {
        match eval_constant(working_set, expr) {
            Ok(val) => match val.coerce_into_string() {
                Ok(s) => (s, expr.span),
                Err(err) => {
                    working_set.error(err.wrap(working_set, call_span));
                    return garbage_pipeline(&[call_span]);
                }
            },
            Err(err) => {
                working_set.error(err.wrap(working_set, call_span));
                return garbage_pipeline(&[call_span]);
            }
        }
    } else {
        working_set.error(ParseError::UnknownState(
            "internal error: Missing required positional after call parsing".into(),
            call_span,
        ));
        return garbage_pipeline(&[call_span]);
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
                        return garbage_pipeline(&[call_span]);
                    }
                },
                Err(err) => {
                    working_set.error(err.wrap(working_set, call_span));
                    return garbage_pipeline(&[call_span]);
                }
            }
        } else {
            working_set.error(ParseError::ExpectedKeyword(
                "as keyword".to_string(),
                kw_expression.span,
            ));
            return garbage_pipeline(&[call_span]);
        }
    } else {
        None
    };

    let Ok(has_prefix) = has_flag_const(working_set, &call, "prefix") else {
        return garbage_pipeline(&[call_span]);
    };
    let Ok(do_reload) = has_flag_const(working_set, &call, "reload") else {
        return garbage_pipeline(&[call_span]);
    };

    let pipeline = Pipeline::from_vec(vec![Expression {
        expr: Expr::Call(call.clone()),
        span: call_span,
        ty: Type::Any,
        custom_completion: None,
    }]);

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

            if let Some(new_name) = new_name {
                if new_name.item != overlay_name {
                    working_set.error(ParseError::CantAddOverlayHelp(
                        format!(
                        "Cannot add overlay as '{}' because it already exists under the name '{}'",
                        new_name.item, overlay_name
                    ),
                        new_name.span,
                    ));
                    return pipeline;
                }
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
            )
        }
    } else {
        (ResolvedImportPattern::new(vec![], vec![], vec![]), vec![])
    };

    if errors.is_empty() {
        working_set.add_overlay(
            final_overlay_name.as_bytes().to_vec(),
            origin_module_id,
            definitions.decls,
            definitions.modules,
            has_prefix,
        );
    } else {
        working_set.parse_errors.extend(errors);
    }

    // Change the call argument to include the Overlay expression with the module ID
    let mut call = call;
    call.set_parser_info(
        "overlay_expr".to_string(),
        Expression {
            expr: Expr::Overlay(if is_module_updated {
                Some(origin_module_id)
            } else {
                None
            }),
            span: overlay_name_span,
            ty: Type::Any,
            custom_completion: None,
        },
    );

    Pipeline::from_vec(vec![Expression {
        expr: Expr::Call(call),
        span: call_span,
        ty: Type::Any,
        custom_completion: None,
    }])
}

pub fn parse_overlay_hide(working_set: &mut StateWorkingSet, call: Box<Call>) -> Pipeline {
    let call_span = call.span();

    let (overlay_name, overlay_name_span) = if let Some(expr) = call.positional_nth(0) {
        match eval_constant(working_set, expr) {
            Ok(val) => match val.coerce_into_string() {
                Ok(s) => (s, expr.span),
                Err(err) => {
                    working_set.error(err.wrap(working_set, call_span));
                    return garbage_pipeline(&[call_span]);
                }
            },
            Err(err) => {
                working_set.error(err.wrap(working_set, call_span));
                return garbage_pipeline(&[call_span]);
            }
        }
    } else {
        (
            String::from_utf8_lossy(working_set.last_overlay_name()).to_string(),
            call_span,
        )
    };

    let Ok(keep_custom) = has_flag_const(working_set, &call, "keep-custom") else {
        return garbage_pipeline(&[call_span]);
    };

    let pipeline = Pipeline::from_vec(vec![Expression {
        expr: Expr::Call(call),
        span: call_span,
        ty: Type::Any,
        custom_completion: None,
    }]);

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
                        working_set.get_span_contents(nu_protocol::span(&spans[(span.0 + 1)..])),
                        spans[span.0 + 1].start,
                        &[],
                        &[],
                        true,
                    );

                    if let Some(parse_error) = parse_error {
                        working_set.error(parse_error)
                    }

                    let rvalue_span = nu_protocol::span(&spans[(span.0 + 1)..]);
                    let rvalue_block = parse_block(working_set, &tokens, rvalue_span, false, true);

                    let output_type = rvalue_block.output_type();

                    let block_id = working_set.add_block(Arc::new(rvalue_block));

                    let rvalue = Expression {
                        expr: Expr::Block(block_id),
                        span: rvalue_span,
                        ty: output_type,
                        custom_completion: None,
                    };

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

                    if let Some(explicit_type) = &explicit_type {
                        if !type_compatible(explicit_type, &rhs_type) {
                            working_set.error(ParseError::TypeMismatch(
                                explicit_type.clone(),
                                rhs_type.clone(),
                                nu_protocol::span(&spans[(span.0 + 1)..]),
                            ));
                        }
                    }

                    if let Some(var_id) = var_id {
                        if explicit_type.is_none() {
                            working_set.set_variable_type(var_id, rhs_type);
                        }
                    }

                    let call = Box::new(Call {
                        decl_id,
                        head: spans[0],
                        arguments: vec![Argument::Positional(lvalue), Argument::Positional(rvalue)],
                        parser_info: HashMap::new(),
                    });

                    return Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: nu_protocol::span(spans),
                        ty: Type::Any,
                        custom_completion: None,
                    }]);
                }
            }
        }
        let ParsedInternalCall { call, output } =
            parse_internal_call(working_set, spans[0], &spans[1..], decl_id);

        return Pipeline::from_vec(vec![Expression {
            expr: Expr::Call(call),
            span: nu_protocol::span(spans),
            ty: output,
            custom_completion: None,
        }]);
    } else {
        working_set.error(ParseError::UnknownState(
            "internal error: let or const statements not found in core language".into(),
            span(spans),
        ))
    }

    working_set.error(ParseError::UnknownState(
        "internal error: let or const statement unparsable".into(),
        span(spans),
    ));

    garbage_pipeline(spans)
}

pub fn parse_const(working_set: &mut StateWorkingSet, spans: &[Span]) -> Pipeline {
    trace!("parsing: const");

    // JT: Disabling check_name because it doesn't work with optional types in the declaration
    // if let Some(span) = check_name(working_set, spans) {
    //     return Pipeline::from_vec(vec![garbage(*span)]);
    // }

    if let Some(decl_id) = working_set.find_decl(b"const") {
        let cmd = working_set.get_decl(decl_id);
        let call_signature = cmd.signature().call_signature();

        if spans.len() >= 4 {
            // This is a bit of by-hand parsing to get around the issue where we want to parse in the reverse order
            // so that the var-id created by the variable isn't visible in the expression that init it
            for span in spans.iter().enumerate() {
                let item = working_set.get_span_contents(*span.1);
                // const x = 'f', = at least start from index 2
                if item == b"=" && spans.len() > (span.0 + 1) && span.0 > 1 {
                    let mut idx = span.0;

                    let rvalue = parse_multispan_value(
                        working_set,
                        spans,
                        &mut idx,
                        &SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::MathExpression)),
                    );
                    if idx < (spans.len() - 1) {
                        working_set
                            .error(ParseError::ExtraPositional(call_signature, spans[idx + 1]));
                    }

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

                    if let Some(explicit_type) = &explicit_type {
                        if !type_compatible(explicit_type, &rhs_type) {
                            working_set.error(ParseError::TypeMismatch(
                                explicit_type.clone(),
                                rhs_type.clone(),
                                nu_protocol::span(&spans[(span.0 + 1)..]),
                            ));
                        }
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
                                            nu_protocol::span(&spans[(span.0 + 1)..]),
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
                        arguments: vec![Argument::Positional(lvalue), Argument::Positional(rvalue)],
                        parser_info: HashMap::new(),
                    });

                    return Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: nu_protocol::span(spans),
                        ty: Type::Any,
                        custom_completion: None,
                    }]);
                }
            }
        }
        let ParsedInternalCall { call, output } =
            parse_internal_call(working_set, spans[0], &spans[1..], decl_id);

        return Pipeline::from_vec(vec![Expression {
            expr: Expr::Call(call),
            span: nu_protocol::span(spans),
            ty: output,
            custom_completion: None,
        }]);
    } else {
        working_set.error(ParseError::UnknownState(
            "internal error: let or const statements not found in core language".into(),
            span(spans),
        ))
    }

    working_set.error(ParseError::UnknownState(
        "internal error: let or const statement unparsable".into(),
        span(spans),
    ));

    garbage_pipeline(spans)
}

pub fn parse_mut(working_set: &mut StateWorkingSet, spans: &[Span]) -> Pipeline {
    trace!("parsing: mut");

    // JT: Disabling check_name because it doesn't work with optional types in the declaration
    // if let Some(span) = check_name(working_set, spans) {
    //     return Pipeline::from_vec(vec![garbage(*span)]);
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
                        working_set.get_span_contents(nu_protocol::span(&spans[(span.0 + 1)..])),
                        spans[span.0 + 1].start,
                        &[],
                        &[],
                        true,
                    );

                    if let Some(parse_error) = parse_error {
                        working_set.error(parse_error);
                    }

                    let rvalue_span = nu_protocol::span(&spans[(span.0 + 1)..]);
                    let rvalue_block = parse_block(working_set, &tokens, rvalue_span, false, true);

                    let output_type = rvalue_block.output_type();

                    let block_id = working_set.add_block(Arc::new(rvalue_block));

                    let rvalue = Expression {
                        expr: Expr::Block(block_id),
                        span: rvalue_span,
                        ty: output_type,
                        custom_completion: None,
                    };

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

                    if let Some(explicit_type) = &explicit_type {
                        if !type_compatible(explicit_type, &rhs_type) {
                            working_set.error(ParseError::TypeMismatch(
                                explicit_type.clone(),
                                rhs_type.clone(),
                                nu_protocol::span(&spans[(span.0 + 1)..]),
                            ));
                        }
                    }

                    if let Some(var_id) = var_id {
                        if explicit_type.is_none() {
                            working_set.set_variable_type(var_id, rhs_type);
                        }
                    }

                    let call = Box::new(Call {
                        decl_id,
                        head: spans[0],
                        arguments: vec![Argument::Positional(lvalue), Argument::Positional(rvalue)],
                        parser_info: HashMap::new(),
                    });

                    return Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: nu_protocol::span(spans),
                        ty: Type::Any,
                        custom_completion: None,
                    }]);
                }
            }
        }
        let ParsedInternalCall { call, output } =
            parse_internal_call(working_set, spans[0], &spans[1..], decl_id);

        return Pipeline::from_vec(vec![Expression {
            expr: Expr::Call(call),
            span: nu_protocol::span(spans),
            ty: output,
            custom_completion: None,
        }]);
    } else {
        working_set.error(ParseError::UnknownState(
            "internal error: let or const statements not found in core language".into(),
            span(spans),
        ))
    }

    working_set.error(ParseError::UnknownState(
        "internal error: let or const statement unparsable".into(),
        span(spans),
    ));

    garbage_pipeline(spans)
}

pub fn parse_source(working_set: &mut StateWorkingSet, lite_command: &LiteCommand) -> Pipeline {
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
            return garbage_pipeline(spans);
        }

        let scoped = name == b"source-env";

        if let Some(decl_id) = working_set.find_decl(name) {
            let cwd = working_set.get_cwd();

            // Is this the right call to be using here?
            // Some of the others (`parse_let`) use it, some of them (`parse_hide`) don't.
            let ParsedInternalCall { call, output } =
                parse_internal_call(working_set, spans[0], &spans[1..], decl_id);

            let Ok(is_help) = has_flag_const(working_set, &call, "help") else {
                return garbage_pipeline(spans);
            };

            if is_help {
                return Pipeline::from_vec(vec![Expression {
                    expr: Expr::Call(call),
                    span: span(spans),
                    ty: output,
                    custom_completion: None,
                }]);
            }

            // Command and one file name
            if spans.len() >= 2 {
                let expr = parse_value(working_set, spans[1], &SyntaxShape::Any);

                let val = match eval_constant(working_set, &expr) {
                    Ok(val) => val,
                    Err(err) => {
                        working_set.error(err.wrap(working_set, span(&spans[1..])));
                        return Pipeline::from_vec(vec![Expression {
                            expr: Expr::Call(call),
                            span: span(&spans[1..]),
                            ty: Type::Any,
                            custom_completion: None,
                        }]);
                    }
                };

                let filename = match val.coerce_into_string() {
                    Ok(s) => s,
                    Err(err) => {
                        working_set.error(err.wrap(working_set, span(&spans[1..])));
                        return Pipeline::from_vec(vec![Expression {
                            expr: Expr::Call(call),
                            span: span(&spans[1..]),
                            ty: Type::Any,
                            custom_completion: None,
                        }]);
                    }
                };

                if let Some(path) = find_in_dirs(&filename, working_set, &cwd, Some(LIB_DIRS_VAR)) {
                    if let Some(contents) = path.read(working_set) {
                        // Add the file to the stack of files being processed.
                        if let Err(e) = working_set.files.push(path.clone().path_buf(), spans[1]) {
                            working_set.error(e);
                            return garbage_pipeline(spans);
                        }

                        // This will load the defs from the file into the
                        // working set, if it was a successful parse.
                        let block = parse(
                            working_set,
                            Some(&path.path().to_string_lossy()),
                            &contents,
                            scoped,
                        );

                        // Remove the file from the stack of files being processed.
                        working_set.files.pop();

                        // Save the block into the working set
                        let block_id = working_set.add_block(block);

                        let mut call_with_block = call;

                        // FIXME: Adding this expression to the positional creates a syntax highlighting error
                        // after writing `source example.nu`
                        call_with_block.set_parser_info(
                            "block_id".to_string(),
                            Expression {
                                expr: Expr::Int(block_id as i64),
                                span: spans[1],
                                ty: Type::Any,
                                custom_completion: None,
                            },
                        );

                        return Pipeline::from_vec(vec![Expression {
                            expr: Expr::Call(call_with_block),
                            span: span(spans),
                            ty: Type::Any,
                            custom_completion: None,
                        }]);
                    }
                } else {
                    working_set.error(ParseError::SourcedFileNotFound(filename, spans[1]));
                }
            }
            return Pipeline::from_vec(vec![Expression {
                expr: Expr::Call(call),
                span: span(spans),
                ty: Type::Any,
                custom_completion: None,
            }]);
        }
    }
    working_set.error(ParseError::UnknownState(
        "internal error: source statement unparsable".into(),
        span(spans),
    ));
    garbage_pipeline(spans)
}

pub fn parse_where_expr(working_set: &mut StateWorkingSet, spans: &[Span]) -> Expression {
    trace!("parsing: where");

    if !spans.is_empty() && working_set.get_span_contents(spans[0]) != b"where" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for 'where' command".into(),
            span(spans),
        ));
        return garbage(span(spans));
    }

    if spans.len() < 2 {
        working_set.error(ParseError::MissingPositional(
            "row condition".into(),
            span(spans),
            "where <row_condition>".into(),
        ));
        return garbage(span(spans));
    }

    let call = match working_set.find_decl(b"where") {
        Some(decl_id) => {
            let ParsedInternalCall { call, output } =
                parse_internal_call(working_set, spans[0], &spans[1..], decl_id);
            let decl = working_set.get_decl(decl_id);

            let call_span = span(spans);

            let starting_error_count = working_set.parse_errors.len();
            check_call(working_set, call_span, &decl.signature(), &call);

            let Ok(is_help) = has_flag_const(working_set, &call, "help") else {
                return garbage(span(spans));
            };

            if starting_error_count != working_set.parse_errors.len() || is_help {
                return Expression {
                    expr: Expr::Call(call),
                    span: call_span,
                    ty: output,
                    custom_completion: None,
                };
            }

            call
        }
        None => {
            working_set.error(ParseError::UnknownState(
                "internal error: 'where' declaration not found".into(),
                span(spans),
            ));
            return garbage(span(spans));
        }
    };

    Expression {
        expr: Expr::Call(call),
        span: span(spans),
        ty: Type::Any,
        custom_completion: None,
    }
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

/// `register` is deprecated and will be removed in 0.94. Use `plugin add` and `plugin use` instead.
#[cfg(feature = "plugin")]
pub fn parse_register(working_set: &mut StateWorkingSet, lite_command: &LiteCommand) -> Pipeline {
    use nu_plugin::{get_signature, PluginDeclaration};
    use nu_protocol::{
        engine::Stack, ErrSpan, ParseWarning, PluginIdentity, PluginRegistryItem, PluginSignature,
        RegisteredPlugin,
    };

    let spans = &lite_command.parts;

    let cwd = working_set.get_cwd();

    // Checking that the function is used with the correct name
    // Maybe this is not necessary but it is a sanity check
    if working_set.get_span_contents(spans[0]) != b"register" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for 'register' function".into(),
            span(spans),
        ));
        return garbage_pipeline(spans);
    }
    if let Some(redirection) = lite_command.redirection.as_ref() {
        working_set.error(redirecting_builtin_error("register", redirection));
        return garbage_pipeline(spans);
    }

    // Parsing the spans and checking that they match the register signature
    // Using a parsed call makes more sense than checking for how many spans are in the call
    // Also, by creating a call, it can be checked if it matches the declaration signature
    let (call, call_span) = match working_set.find_decl(b"register") {
        None => {
            working_set.error(ParseError::UnknownState(
                "internal error: Register declaration not found".into(),
                span(spans),
            ));
            return garbage_pipeline(spans);
        }
        Some(decl_id) => {
            let ParsedInternalCall { call, output } =
                parse_internal_call(working_set, spans[0], &spans[1..], decl_id);
            let decl = working_set.get_decl(decl_id);

            let call_span = span(spans);

            let starting_error_count = working_set.parse_errors.len();
            check_call(working_set, call_span, &decl.signature(), &call);

            let Ok(is_help) = has_flag_const(working_set, &call, "help") else {
                return garbage_pipeline(spans);
            };

            if starting_error_count != working_set.parse_errors.len() || is_help {
                return Pipeline::from_vec(vec![Expression {
                    expr: Expr::Call(call),
                    span: call_span,
                    ty: output,
                    custom_completion: None,
                }]);
            }

            (call, call_span)
        }
    };

    // Now that the call is parsed, add the deprecation warning
    working_set
        .parse_warnings
        .push(ParseWarning::DeprecatedWarning {
            old_command: "register".into(),
            new_suggestion: "use `plugin add` and `plugin use`".into(),
            span: call.head,
            url: "https://www.nushell.sh/book/plugins.html".into(),
        });

    // Extracting the required arguments from the call and keeping them together in a tuple
    let arguments = call
        .positional_nth(0)
        .map(|expr| {
            let val =
                eval_constant(working_set, expr).map_err(|err| err.wrap(working_set, call.head))?;
            let filename = val
                .coerce_into_string()
                .map_err(|err| err.wrap(working_set, call.head))?;

            let Some(path) = find_in_dirs(&filename, working_set, &cwd, Some(PLUGIN_DIRS_VAR))
            else {
                return Err(ParseError::RegisteredFileNotFound(filename, expr.span));
            };

            if path.exists() && path.is_file() {
                Ok((path, expr.span))
            } else {
                Err(ParseError::RegisteredFileNotFound(filename, expr.span))
            }
        })
        .expect("required positional has being checked");

    // Signature is an optional value from the call and will be used to decide if
    // the plugin is called to get the signatures or to use the given signature
    let signature = call.positional_nth(1).map(|expr| {
        let signature = working_set.get_span_contents(expr.span);
        serde_json::from_slice::<PluginSignature>(signature).map_err(|e| {
            ParseError::LabeledError(
                "Signature deserialization error".into(),
                format!("unable to deserialize signature: {e}"),
                spans[0],
            )
        })
    });

    // Shell is another optional value used as base to call shell to plugins
    let shell = call.get_flag_expr("shell").map(|expr| {
        let shell_expr = working_set.get_span_contents(expr.span);

        String::from_utf8(shell_expr.to_vec())
            .map_err(|_| ParseError::NonUtf8(expr.span))
            .and_then(|name| {
                canonicalize_with(&name, cwd)
                    .map_err(|_| ParseError::RegisteredFileNotFound(name, expr.span))
            })
            .and_then(|path| {
                if path.exists() & path.is_file() {
                    Ok(path)
                } else {
                    Err(ParseError::RegisteredFileNotFound(
                        format!("{path:?}"),
                        expr.span,
                    ))
                }
            })
    });

    let shell = match shell {
        None => None,
        Some(path) => match path {
            Ok(path) => Some(path),
            Err(err) => {
                working_set.error(err);
                return Pipeline::from_vec(vec![Expression {
                    expr: Expr::Call(call),
                    span: call_span,
                    ty: Type::Any,
                    custom_completion: None,
                }]);
            }
        },
    };

    // We need the current environment variables for `python` based plugins
    // Or we'll likely have a problem when a plugin is implemented in a virtual Python environment.
    let get_envs = || {
        let stack = Stack::new().capture();
        nu_engine::env::env_to_strings(working_set.permanent_state, &stack)
    };

    let error = arguments.and_then(|(path, path_span)| {
        let path = path.path_buf();

        // Create the plugin identity. This validates that the plugin name starts with `nu_plugin_`
        let identity = PluginIdentity::new(path, shell).err_span(path_span)?;

        let plugin = nu_plugin::add_plugin_to_working_set(working_set, &identity)
            .map_err(|err| err.wrap(working_set, call.head))?;

        let signatures = signature.map_or_else(
            || {
                // It's important that the plugin is restarted if we're going to get signatures
                //
                // The user would expect that `register` would always run the binary to get new
                // signatures, in case it was replaced with an updated binary
                plugin.reset().map_err(|err| {
                    ParseError::LabeledError(
                        "Failed to restart plugin to get new signatures".into(),
                        err.to_string(),
                        spans[0],
                    )
                })?;

                let signatures = get_signature(plugin.clone(), get_envs).map_err(|err| {
                    log::warn!("Error getting signatures: {err:?}");
                    ParseError::LabeledError(
                        "Error getting signatures".into(),
                        err.to_string(),
                        spans[0],
                    )
                });

                if let Ok(ref signatures) = signatures {
                    // Add the loaded plugin to the delta
                    working_set.update_plugin_registry(PluginRegistryItem::new(
                        &identity,
                        signatures.clone(),
                    ));
                }

                signatures
            },
            |sig| sig.map(|sig| vec![sig]),
        )?;

        for signature in signatures {
            // create plugin command declaration (need struct impl Command)
            // store declaration in working set
            let plugin_decl = PluginDeclaration::new(plugin.clone(), signature);

            working_set.add_decl(Box::new(plugin_decl));
        }

        Ok(())
    });

    if let Err(err) = error {
        working_set.error(err);
    }

    Pipeline::from_vec(vec![Expression {
        expr: Expr::Call(call),
        span: call_span,
        ty: Type::Nothing,
        custom_completion: None,
    }])
}

#[cfg(feature = "plugin")]
pub fn parse_plugin_use(working_set: &mut StateWorkingSet, call: Box<Call>) -> Pipeline {
    use nu_protocol::{FromValue, PluginRegistryFile};

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
        nu_plugin::load_plugin_registry_item(working_set, plugin_item, Some(call.head))
            .map_err(|err| err.wrap(working_set, call.head))?;

        Ok(())
    })() {
        working_set.error(err);
    }

    let call_span = call.span();

    Pipeline::from_vec(vec![Expression {
        expr: Expr::Call(call),
        span: call_span,
        ty: Type::Nothing,
        custom_completion: None,
    }])
}

pub fn find_dirs_var(working_set: &StateWorkingSet, var_name: &str) -> Option<VarId> {
    working_set
        .find_variable(format!("${}", var_name).as_bytes())
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
                                if let Ok(dir_abs) = canonicalize_with(dir, actual_cwd) {
                                    if let Ok(path) = canonicalize_with(filename, dir_abs) {
                                        return Some(path);
                                    }
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
    decl_name: &str,
) -> Option<ParseError> {
    let name = working_set.get_span_contents(name_span);

    let extract_span = |delim: u8| {
        // it is okay to unwrap because we know the slice contains the byte
        let (idx, _) = name
            .iter()
            .find_position(|c| **c == delim)
            .unwrap_or((name.len(), &b' '));
        let param_span = Span::new(name_span.start + idx - 1, name_span.start + idx - 1);
        let error = ParseError::LabeledErrorWithHelp{
            error: "no space between name and parameters".into(),
            label: "expected space".into(),
            help: format!("consider adding a space between the `{decl_name}` command's name and its parameters"),
            span: param_span,
            };
        Some(error)
    };

    if name.contains(&b'[') {
        extract_span(b'[')
    } else if name.contains(&b'(') {
        extract_span(b'(')
    } else {
        None
    }
}

/// Run has_flag_const and push possible error to working_set
fn has_flag_const(working_set: &mut StateWorkingSet, call: &Call, name: &str) -> Result<bool, ()> {
    call.has_flag_const(working_set, name).map_err(|err| {
        working_set.error(err.wrap(working_set, call.span()));
    })
}
