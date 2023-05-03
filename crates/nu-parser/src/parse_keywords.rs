use log::trace;
use nu_path::canonicalize_with;
use nu_protocol::{
    ast::{
        Argument, Block, Call, Expr, Expression, ImportPattern, ImportPatternHead,
        ImportPatternMember, Pipeline, PipelineElement,
    },
    engine::{StateWorkingSet, DEFAULT_OVERLAY_NAME},
    span, Alias, BlockId, Exportable, Module, ParseError, PositionalArg, Span, Spanned,
    SyntaxShape, Type, VarId,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub const LIB_DIRS_VAR: &str = "NU_LIB_DIRS";
#[cfg(feature = "plugin")]
pub const PLUGIN_DIRS_VAR: &str = "NU_PLUGIN_DIRS";

use crate::{
    eval::{eval_constant, value_as_string},
    is_math_expression_like,
    known_external::KnownExternal,
    lex,
    lite_parser::{lite_parse, LiteCommand, LiteElement},
    parser::{
        check_call, check_name, garbage, garbage_pipeline, parse, parse_call, parse_expression,
        parse_import_pattern, parse_internal_call, parse_multispan_value, parse_signature,
        parse_string, parse_value, parse_var_with_opt_type, trim_quotes, ParsedInternalCall,
    },
    unescape_unquote_string, Token, TokenContents,
};

/// These parser keywords can be aliased
pub const ALIASABLE_PARSER_KEYWORDS: &[&[u8]] = &[b"overlay hide", b"overlay new", b"overlay use"];

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
];

/// Check whether spans start with a parser keyword that can be aliased
pub fn is_unaliasable_parser_keyword(working_set: &StateWorkingSet, spans: &[Span]) -> bool {
    // try two words
    if let (Some(span1), Some(span2)) = (spans.get(0), spans.get(1)) {
        let cmd_name = working_set.get_span_contents(span(&[*span1, *span2]));
        return UNALIASABLE_PARSER_KEYWORDS.contains(&cmd_name);
    }

    // try one word
    if let Some(span1) = spans.get(0) {
        let cmd_name = working_set.get_span_contents(*span1);
        UNALIASABLE_PARSER_KEYWORDS.contains(&cmd_name)
    } else {
        false
    }
}

/// This is a new more compact method of calling parse_xxx() functions without repeating the
/// parse_call() in each function. Remaining keywords can be moved here.
pub fn parse_keyword(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
    is_subexpression: bool,
) -> Pipeline {
    let call_expr = parse_call(
        working_set,
        &lite_command.parts,
        lite_command.parts[0],
        is_subexpression,
    );

    // if err.is_some() {
    //     return (Pipeline::from_vec(vec![call_expr]), err);
    // }

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
            _ => Pipeline::from_vec(vec![call_expr]),
        }
    } else {
        Pipeline::from_vec(vec![call_expr])
    }
}

pub fn parse_def_predecl(working_set: &mut StateWorkingSet, spans: &[Span]) {
    let name = working_set.get_span_contents(spans[0]);

    // handle "export def" same as "def"
    let (name, spans) = if name == b"export" && spans.len() >= 2 {
        (working_set.get_span_contents(spans[1]), &spans[1..])
    } else {
        (name, spans)
    };

    if (name == b"def" || name == b"def-env") && spans.len() >= 4 {
        let starting_error_count = working_set.parse_errors.len();
        let name = working_set.get_span_contents(spans[1]);
        let name = trim_quotes(name);
        let name = String::from_utf8_lossy(name).to_string();

        working_set.enter_scope();
        // FIXME: because parse_signature will update the scope with the variables it sees
        // we end up parsing the signature twice per def. The first time is during the predecl
        // so that we can see the types that are part of the signature, which we need for parsing.
        // The second time is when we actually parse the body itworking_set.
        // We can't reuse the first time because the variables that are created during parse_signature
        // are lost when we exit the scope below.
        let sig = parse_signature(working_set, spans[2]);
        working_set.parse_errors.truncate(starting_error_count);

        let signature = sig.as_signature();
        working_set.exit_scope();
        if name.contains('#')
            || name.contains('^')
            || name.parse::<bytesize::ByteSize>().is_ok()
            || name.parse::<f64>().is_ok()
        {
            working_set.error(ParseError::CommandDefNotValid(spans[1]));
            return;
        }

        if let Some(mut signature) = signature {
            signature.name = name;
            let decl = signature.predeclare();

            if working_set.add_predecl(decl).is_some() {
                working_set.error(ParseError::DuplicateCommandDef(spans[1]));
            }
        }
    } else if name == b"extern" && spans.len() >= 3 {
        let name_expr = parse_string(working_set, spans[1]);
        let name = name_expr.as_string();

        working_set.enter_scope();
        // FIXME: because parse_signature will update the scope with the variables it sees
        // we end up parsing the signature twice per def. The first time is during the predecl
        // so that we can see the types that are part of the signature, which we need for parsing.
        // The second time is when we actually parse the body itworking_set.
        // We can't reuse the first time because the variables that are created during parse_signature
        // are lost when we exit the scope below.
        let sig = parse_signature(working_set, spans[2]);
        let signature = sig.as_signature();
        working_set.exit_scope();

        if let (Some(name), Some(mut signature)) = (name, signature) {
            if name.contains('#')
                || name.parse::<bytesize::ByteSize>().is_ok()
                || name.parse::<f64>().is_ok()
            {
                working_set.error(ParseError::CommandDefNotValid(spans[1]));
                return;
            }

            signature.name = name.clone();
            //let decl = signature.predeclare();
            let decl = KnownExternal {
                name,
                usage: "run external command".into(),
                signature,
            };

            if working_set.add_predecl(Box::new(decl)).is_some() {
                working_set.error(ParseError::DuplicateCommandDef(spans[1]));
                return;
            }
        }
    }
}

pub fn parse_for(working_set: &mut StateWorkingSet, spans: &[Span]) -> Expression {
    // Checking that the function is used with the correct name
    // Maybe this is not necessary but it is a sanity check
    if working_set.get_span_contents(spans[0]) != b"for" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for 'for' function".into(),
            span(spans),
        ));
        return garbage(spans[0]);
    }

    // Parsing the spans and checking that they match the register signature
    // Using a parsed call makes more sense than checking for how many spans are in the call
    // Also, by creating a call, it can be checked if it matches the declaration signature
    let (call, call_span) = match working_set.find_decl(b"for", &Type::Any) {
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
            if starting_error_count != working_set.parse_errors.len() || call.has_flag("help") {
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
    let block = call.positional_nth(2).expect("for call already checked");

    if let (Some(var_id), Some(block_id)) = (&var_decl.as_var(), block.as_block()) {
        let block = working_set.get_block_mut(block_id);

        block.signature.required_positional.insert(
            0,
            PositionalArg {
                name: String::new(),
                desc: String::new(),
                shape: SyntaxShape::Any,
                var_id: Some(*var_id),
                default_value: None,
            },
        );
    }

    Expression {
        expr: Expr::Call(call),
        span: call_span,
        ty: Type::Any,
        custom_completion: None,
    }
}

pub fn parse_def(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
    module_name: Option<&[u8]>,
) -> Pipeline {
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
    if def_call != b"def" && def_call != b"def-env" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for def function".into(),
            span(spans),
        ));
        return garbage_pipeline(spans);
    }

    // Parsing the spans and checking that they match the register signature
    // Using a parsed call makes more sense than checking for how many spans are in the call
    // Also, by creating a call, it can be checked if it matches the declaration signature
    let (call, call_span) = match working_set.find_decl(&def_call, &Type::Any) {
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
                        expr: Expr::Block(block_id),
                        ..
                    }
                    | Expression {
                        expr: Expr::RowCondition(block_id),
                        ..
                    } => {
                        let block = working_set.get_block_mut(*block_id);

                        block.signature = Box::new(sig.clone());
                    }
                    _ => {}
                }
            }

            let starting_error_count = working_set.parse_errors.len();
            check_call(working_set, call_span, &sig, &call);
            working_set.parse_errors.append(&mut new_errors);
            if starting_error_count != working_set.parse_errors.len() || call.has_flag("help") {
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

        name
    } else {
        working_set.error(ParseError::UnknownState(
            "Could not get string from string expression".into(),
            name_expr.span,
        ));
        return garbage_pipeline(spans);
    };

    if let (Some(mut signature), Some(block_id)) = (sig.as_signature(), block.as_block()) {
        if let Some(decl_id) = working_set.find_predecl(name.as_bytes()) {
            let declaration = working_set.get_decl_mut(decl_id);

            signature.name = name.clone();
            *signature = signature.add_help();
            signature.usage = usage;
            signature.extra_usage = extra_usage;

            *declaration = signature.clone().into_block_command(block_id);

            let mut block = working_set.get_block_mut(block_id);
            let calls_itself = block_calls_itself(block, decl_id);
            block.recursive = Some(calls_itself);
            block.signature = signature;
            block.redirect_env = def_call == b"def-env";
        } else {
            working_set.error(ParseError::InternalError(
                "Predeclaration failed to add declaration".into(),
                name_expr.span,
            ));
        };
    }

    // It's OK if it returns None: The decl was already merged in previous parse pass.
    working_set.merge_predecl(name.as_bytes());

    Pipeline::from_vec(vec![Expression {
        expr: Expr::Call(call),
        span: call_span,
        ty: Type::Any,
        custom_completion: None,
    }])
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
        if spans.len() > 1 && working_set.get_span_contents(spans[0]) == b"export" {
            (spans[1], 2)
        } else {
            (spans[0], 1)
        };

    let extern_call = working_set.get_span_contents(name_span).to_vec();
    if extern_call != b"extern" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for extern function".into(),
            span(spans),
        ));
        return garbage_pipeline(spans);
    }

    // Parsing the spans and checking that they match the register signature
    // Using a parsed call makes more sense than checking for how many spans are in the call
    // Also, by creating a call, it can be checked if it matches the declaration signature
    let (call, call_span) = match working_set.find_decl(&extern_call, &Type::Any) {
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

                signature.name = external_name.clone();
                signature.usage = usage.clone();
                signature.extra_usage = extra_usage.clone();
                signature.allows_unknown_args = true;

                if let Some(block_id) = body.and_then(|x| x.as_block()) {
                    if signature.rest_positional.is_none() {
                        working_set.error(ParseError::InternalError(
                            "Extern block must have a rest positional argument".into(),
                            name_expr.span,
                        ));
                    } else {
                        *declaration = signature.clone().into_block_command(block_id);

                        let block = working_set.get_block_mut(block_id);
                        let calls_itself = block_calls_itself(block, decl_id);
                        block.recursive = Some(calls_itself);
                        block.signature = signature;
                    }
                } else {
                    let decl = KnownExternal {
                        name: external_name,
                        usage: [usage, extra_usage].join("\n"),
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

fn block_calls_itself(block: &Block, decl_id: usize) -> bool {
    block.pipelines.iter().any(|pipeline| {
        pipeline
            .elements
            .iter()
            .any(|pipe_element| match pipe_element {
                PipelineElement::Expression(
                    _,
                    Expression {
                        expr: Expr::Call(call_expr),
                        ..
                    },
                ) => {
                    if call_expr.decl_id == decl_id {
                        return true;
                    }
                    call_expr.arguments.iter().any(|arg| match arg {
                        Argument::Positional(Expression { expr, .. }) => match expr {
                            Expr::Keyword(.., expr) => {
                                let expr = expr.as_ref();
                                let Expression { expr, .. } = expr;
                                match expr {
                                    Expr::Call(call_expr2) => call_expr2.decl_id == decl_id,
                                    _ => false,
                                }
                            }
                            Expr::Call(call_expr2) => call_expr2.decl_id == decl_id,
                            _ => false,
                        },
                        _ => false,
                    })
                }
                _ => false,
            })
    })
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

    if let Some(span) = check_name(working_set, spans) {
        return Pipeline::from_vec(vec![garbage(*span)]);
    }

    if let Some(decl_id) = working_set.find_decl(b"alias", &Type::Any) {
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

        let has_help_flag = alias_call.has_flag("help");

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
                let expr = parse_expression(working_set, replacement_spans, false);
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
            let expr = parse_call(
                working_set,
                replacement_spans,
                replacement_spans[0],
                false, // TODO: Should this be set properly???
            );

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

            let decl = Alias {
                name: alias_name,
                command,
                wrapped_call,
            };

            working_set.add_decl(Box::new(decl));
        }

        if spans.len() < 4 {
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
            b"alias" | b"def" | b"def-env" | b"extern" | b"use" => [b"export ", sub].concat(),
            _ => b"export".to_vec(),
        }
    } else {
        b"export".to_vec()
    };

    if let Some(decl_id) = working_set.find_decl(&full_name, &Type::Any) {
        let ParsedInternalCall { call, output, .. } = parse_internal_call(
            working_set,
            if full_name == b"export" {
                lite_command.parts[0]
            } else {
                span(&lite_command.parts[0..2])
            },
            if full_name == b"export" {
                &lite_command.parts[1..]
            } else {
                &lite_command.parts[2..]
            },
            decl_id,
        );

        let decl = working_set.get_decl(decl_id);

        let starting_error_count = working_set.parse_errors.len();
        check_call(working_set, call_span, &decl.signature(), &call);
        if starting_error_count != working_set.parse_errors.len() || call.has_flag("help") {
            return Pipeline::from_vec(vec![Expression {
                expr: Expr::Call(call),
                span: call_span,
                ty: output,
                custom_completion: None,
            }]);
        }
    } else {
        working_set.error(ParseError::UnknownState(
            format!(
                "internal error: '{}' declaration not found",
                String::from_utf8_lossy(&full_name)
            ),
            span(&lite_command.parts),
        ));
        return garbage_pipeline(&lite_command.parts);
    };

    if &full_name == b"export" {
        // export by itself is meaningless
        working_set.error(ParseError::UnexpectedKeyword(
            "export".into(),
            lite_command.parts[0],
        ));
        return garbage_pipeline(&lite_command.parts);
    }

    match full_name.as_slice() {
        b"export alias" => parse_alias(working_set, lite_command, None),
        b"export def" | b"export def-env" => parse_def(working_set, lite_command, None),
        b"export use" => {
            let (pipeline, _) = parse_use(working_set, &lite_command.parts);
            pipeline
        }
        b"export extern" => parse_extern(working_set, lite_command, None),
        _ => {
            working_set.error(ParseError::UnexpectedKeyword(
                String::from_utf8_lossy(&full_name).to_string(),
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

    let export_span = if let Some(sp) = spans.get(0) {
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

    let Some(export_decl_id) = working_set.find_decl(b"export", &Type::Any) else {
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
        redirect_stdout: true,
        redirect_stderr: false,
        parser_info: HashMap::new(),
    });

    let exportables = if let Some(kw_span) = spans.get(1) {
        let kw_name = working_set.get_span_contents(*kw_span);
        match kw_name {
            b"def" => {
                let lite_command = LiteCommand {
                    comments: lite_command.comments.clone(),
                    parts: spans[1..].to_vec(),
                };
                let pipeline = parse_def(working_set, &lite_command, Some(module_name));

                let export_def_decl_id =
                    if let Some(id) = working_set.find_decl(b"export def", &Type::Any) {
                        id
                    } else {
                        working_set.error(ParseError::InternalError(
                            "missing 'export def' command".into(),
                            export_span,
                        ));
                        return (garbage_pipeline(spans), vec![]);
                    };

                // Trying to warp the 'def' call into the 'export def' in a very clumsy way
                if let Some(PipelineElement::Expression(
                    _,
                    Expression {
                        expr: Expr::Call(ref def_call),
                        ..
                    },
                )) = pipeline.elements.get(0)
                {
                    call = def_call.clone();

                    call.head = span(&spans[0..=1]);
                    call.decl_id = export_def_decl_id;
                } else {
                    working_set.error(ParseError::InternalError(
                        "unexpected output from parsing a definition".into(),
                        span(&spans[1..]),
                    ));
                };

                let mut result = vec![];

                if let Some(decl_name_span) = spans.get(2) {
                    let decl_name = working_set.get_span_contents(*decl_name_span);
                    let decl_name = trim_quotes(decl_name);

                    if let Some(decl_id) = working_set.find_decl(decl_name, &Type::Any) {
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
                }

                result
            }
            b"def-env" => {
                let lite_command = LiteCommand {
                    comments: lite_command.comments.clone(),
                    parts: spans[1..].to_vec(),
                };
                let pipeline = parse_def(working_set, &lite_command, Some(module_name));

                let export_def_decl_id =
                    if let Some(id) = working_set.find_decl(b"export def-env", &Type::Any) {
                        id
                    } else {
                        working_set.error(ParseError::InternalError(
                            "missing 'export def-env' command".into(),
                            export_span,
                        ));
                        return (garbage_pipeline(spans), vec![]);
                    };

                // Trying to warp the 'def' call into the 'export def' in a very clumsy way
                if let Some(PipelineElement::Expression(
                    _,
                    Expression {
                        expr: Expr::Call(ref def_call),
                        ..
                    },
                )) = pipeline.elements.get(0)
                {
                    call = def_call.clone();

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

                if let Some(decl_id) = working_set.find_decl(decl_name, &Type::Any) {
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
            b"extern" => {
                let lite_command = LiteCommand {
                    comments: lite_command.comments.clone(),
                    parts: spans[1..].to_vec(),
                };
                let pipeline = parse_extern(working_set, &lite_command, Some(module_name));

                let export_def_decl_id =
                    if let Some(id) = working_set.find_decl(b"export extern", &Type::Any) {
                        id
                    } else {
                        working_set.error(ParseError::InternalError(
                            "missing 'export extern' command".into(),
                            export_span,
                        ));
                        return (garbage_pipeline(spans), vec![]);
                    };

                // Trying to warp the 'def' call into the 'export def' in a very clumsy way
                if let Some(PipelineElement::Expression(
                    _,
                    Expression {
                        expr: Expr::Call(ref def_call),
                        ..
                    },
                )) = pipeline.elements.get(0)
                {
                    call = def_call.clone();

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

                if let Some(decl_id) = working_set.find_decl(decl_name, &Type::Any) {
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
                };
                let pipeline = parse_alias(working_set, &lite_command, Some(module_name));

                let export_alias_decl_id =
                    if let Some(id) = working_set.find_decl(b"export alias", &Type::Any) {
                        id
                    } else {
                        working_set.error(ParseError::InternalError(
                            "missing 'export alias' command".into(),
                            export_span,
                        ));
                        return (garbage_pipeline(spans), vec![]);
                    };

                // Trying to warp the 'alias' call into the 'export alias' in a very clumsy way
                if let Some(PipelineElement::Expression(
                    _,
                    Expression {
                        expr: Expr::Call(ref alias_call),
                        ..
                    },
                )) = pipeline.elements.get(0)
                {
                    call = alias_call.clone();

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

                if let Some(alias_id) = working_set.find_decl(alias_name, &Type::Any) {
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
                };
                let (pipeline, exportables) = parse_use(working_set, &lite_command.parts);

                let export_use_decl_id =
                    if let Some(id) = working_set.find_decl(b"export use", &Type::Any) {
                        id
                    } else {
                        working_set.error(ParseError::InternalError(
                            "missing 'export use' command".into(),
                            export_span,
                        ));
                        return (garbage_pipeline(spans), vec![]);
                    };

                // Trying to warp the 'use' call into the 'export use' in a very clumsy way
                if let Some(PipelineElement::Expression(
                    _,
                    Expression {
                        expr: Expr::Call(ref use_call),
                        ..
                    },
                )) = pipeline.elements.get(0)
                {
                    call = use_call.clone();

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
            _ => {
                working_set.error(ParseError::Expected(
                    // TODO: Fill in more keywords as they come
                    "def, def-env, alias, use, or extern keyword".into(),
                    spans[1],
                ));

                vec![]
            }
        }
    } else {
        working_set.error(ParseError::MissingPositional(
            "def, def-env, alias, use, or extern keyword".into(), // TODO: keep filling more keywords as they come
            Span::new(export_span.end, export_span.end),
            "`def`, `def-env`, `alias`, use, or `extern` keyword.".to_string(),
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

    let call = match working_set.find_decl(b"export-env", &Type::Any) {
        Some(decl_id) => {
            let ParsedInternalCall { call, output } =
                parse_internal_call(working_set, spans[0], &[spans[1]], decl_id);
            let decl = working_set.get_decl(decl_id);

            let call_span = span(spans);

            let starting_error_count = working_set.parse_errors.len();
            check_call(working_set, call_span, &decl.signature(), &call);
            if starting_error_count != working_set.parse_errors.len() || call.has_flag("help") {
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
            if let LiteElement::Command(_, command) = &pipeline.commands[0] {
                parse_def_predecl(working_set, &command.parts);
            }
        }
    }

    let mut module = Module::from_span(module_name.to_vec(), span);

    let mut block = Block::new_with_capacity(output.block.len());

    for pipeline in output.block.iter() {
        if pipeline.commands.len() == 1 {
            match &pipeline.commands[0] {
                LiteElement::Command(_, command) => {
                    let name = working_set.get_span_contents(command.parts[0]);

                    match name {
                        b"def" | b"def-env" => {
                            block.pipelines.push(parse_def(
                                working_set,
                                command,
                                None, // using commands named as the module locally is OK
                            ))
                        }
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
                            let (pipeline, _) = parse_use(working_set, &command.parts);

                            block.pipelines.push(pipeline)
                        }
                        b"export" => {
                            let (pipe, exportables) =
                                parse_export_in_module(working_set, command, module_name);

                            for exportable in exportables {
                                match exportable {
                                    Exportable::Decl { name, id } => {
                                        if &name == b"main" {
                                            module.main = Some(id);
                                        } else {
                                            module.add_decl(name, id);
                                        }
                                    }
                                }
                            }

                            block.pipelines.push(pipe)
                        }
                        b"export-env" => {
                            let (pipe, maybe_env_block) =
                                parse_export_env(working_set, &command.parts);

                            if let Some(block_id) = maybe_env_block {
                                module.add_env_block(block_id);
                            }

                            block.pipelines.push(pipe)
                        }
                        _ => {
                            working_set.error(ParseError::ExpectedKeyword(
                                "def or export keyword".into(),
                                command.parts[0],
                            ));

                            block.pipelines.push(garbage_pipeline(&command.parts))
                        }
                    }
                }
                LiteElement::Redirection(_, _, command) => {
                    block.pipelines.push(garbage_pipeline(&command.parts))
                }
                LiteElement::SeparateRedirection {
                    out: (_, command), ..
                } => block.pipelines.push(garbage_pipeline(&command.parts)),
            }
        } else {
            working_set.error(ParseError::Expected("not a pipeline".into(), span));
            block.pipelines.push(garbage_pipeline(&[span]))
        }
    }

    working_set.exit_scope();

    (block, module, module_comments)
}

pub fn parse_module(working_set: &mut StateWorkingSet, lite_command: &LiteCommand) -> Pipeline {
    // TODO: Currently, module is closing over its parent scope (i.e., defs in the parent scope are
    // visible and usable in this module's scope). We want to disable that for files.

    let spans = &lite_command.parts;
    let mut module_comments = lite_command.comments.clone();

    let bytes = working_set.get_span_contents(spans[0]);

    if bytes == b"module" && spans.len() >= 3 {
        let module_name_expr = parse_string(working_set, spans[1]);

        let module_name = module_name_expr
            .as_string()
            .expect("internal error: module name is not a string");

        let block_span = spans[2];
        let block_bytes = working_set.get_span_contents(block_span);
        let mut start = block_span.start;
        let mut end = block_span.end;

        if block_bytes.starts_with(b"{") {
            start += 1;
        } else {
            working_set.error(ParseError::Expected("block".into(), block_span));
            return garbage_pipeline(spans);
        }

        if block_bytes.ends_with(b"}") {
            end -= 1;
        } else {
            working_set.error(ParseError::Unclosed("}".into(), Span::new(end, end)));
        }

        let block_span = Span::new(start, end);

        let (block, module, inner_comments) =
            parse_module_block(working_set, block_span, module_name.as_bytes());

        let block_id = working_set.add_block(block);

        module_comments.extend(inner_comments);
        let _ = working_set.add_module(&module_name, module, module_comments);

        let block_expr = Expression {
            expr: Expr::Block(block_id),
            span: block_span,
            ty: Type::Block,
            custom_completion: None,
        };

        let module_decl_id = working_set
            .find_decl(b"module", &Type::Any)
            .expect("internal error: missing module command");

        let call = Box::new(Call {
            head: spans[0],
            decl_id: module_decl_id,
            arguments: vec![
                Argument::Positional(module_name_expr),
                Argument::Positional(block_expr),
            ],
            redirect_stdout: true,
            redirect_stderr: false,
            parser_info: HashMap::new(),
        });

        Pipeline::from_vec(vec![Expression {
            expr: Expr::Call(call),
            span: span(spans),
            ty: Type::Any,
            custom_completion: None,
        }])
    } else {
        working_set.error(ParseError::UnknownState(
            "Expected structure: module <name> {}".into(),
            span(spans),
        ));

        garbage_pipeline(spans)
    }
}

pub fn parse_use(working_set: &mut StateWorkingSet, spans: &[Span]) -> (Pipeline, Vec<Exportable>) {
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

    let (call, call_span, args_spans) = match working_set.find_decl(b"use", &Type::Any) {
        Some(decl_id) => {
            let (command_spans, rest_spans) = spans.split_at(split_id);

            let ParsedInternalCall { call, output } =
                parse_internal_call(working_set, span(command_spans), rest_spans, decl_id);
            let decl = working_set.get_decl(decl_id);

            let call_span = span(spans);

            let starting_error_count = working_set.parse_errors.len();
            check_call(working_set, call_span, &decl.signature(), &call);
            if starting_error_count != working_set.parse_errors.len() || call.has_flag("help") {
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

    let cwd = working_set.get_cwd();

    // TODO: Add checking for importing too long import patterns, e.g.:
    // > use spam foo non existent names here do not throw error
    let (import_pattern, module) = if let Some(module_id) = import_pattern.head.id {
        (import_pattern, working_set.get_module(module_id).clone())
    } else {
        // It could be a file
        // TODO: Do not close over when loading module from file?

        let starting_error_count = working_set.parse_errors.len();
        let (module_filename, err) =
            unescape_unquote_string(&import_pattern.head.name, import_pattern.head.span);
        if let Some(err) = err {
            working_set.error(err);
        }

        if starting_error_count == working_set.parse_errors.len() {
            if let Some(module_path) =
                find_in_dirs(&module_filename, working_set, &cwd, LIB_DIRS_VAR)
            {
                if let Some(i) = working_set
                    .parsed_module_files
                    .iter()
                    .rposition(|p| p == &module_path)
                {
                    let mut files: Vec<String> = working_set
                        .parsed_module_files
                        .split_off(i)
                        .iter()
                        .map(|p| p.to_string_lossy().to_string())
                        .collect();

                    files.push(module_path.to_string_lossy().to_string());

                    let msg = files.join("\nuses ");

                    working_set.error(ParseError::CyclicalModuleImport(
                        msg,
                        import_pattern.head.span,
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
                }

                let module_name = if let Some(stem) = module_path.file_stem() {
                    stem.to_string_lossy().to_string()
                } else {
                    working_set.error(ParseError::ModuleNotFound(import_pattern.head.span));
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

                if let Ok(contents) = std::fs::read(&module_path) {
                    let file_id =
                        working_set.add_file(module_path.to_string_lossy().to_string(), &contents);
                    let new_span = working_set.get_span_for_file(file_id);

                    // Change the currently parsed directory
                    let prev_currently_parsed_cwd = if let Some(parent) = module_path.parent() {
                        let prev = working_set.currently_parsed_cwd.clone();

                        working_set.currently_parsed_cwd = Some(parent.into());

                        prev
                    } else {
                        working_set.currently_parsed_cwd.clone()
                    };

                    // Add the file to the stack of parsed module files
                    working_set.parsed_module_files.push(module_path);

                    // Parse the module
                    let (block, module, module_comments) =
                        parse_module_block(working_set, new_span, module_name.as_bytes());

                    // Remove the file from the stack of parsed module files
                    working_set.parsed_module_files.pop();

                    // Restore the currently parsed directory back
                    working_set.currently_parsed_cwd = prev_currently_parsed_cwd;

                    let _ = working_set.add_block(block);
                    let module_id =
                        working_set.add_module(&module_name, module.clone(), module_comments);

                    (
                        ImportPattern {
                            head: ImportPatternHead {
                                name: module_name.into(),
                                id: Some(module_id),
                                span: import_pattern.head.span,
                            },
                            members: import_pattern.members,
                            hidden: HashSet::new(),
                        },
                        module,
                    )
                } else {
                    working_set.error(ParseError::ModuleNotFound(import_pattern.head.span));
                    return (
                        Pipeline::from_vec(vec![Expression {
                            expr: Expr::Call(call),
                            span: call_span,
                            ty: Type::Any,
                            custom_completion: None,
                        }]),
                        vec![],
                    );
                }
            } else {
                working_set.error(ParseError::ModuleNotFound(import_pattern.head.span));
                return (
                    Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: span(spans),
                        ty: Type::Any,
                        custom_completion: None,
                    }]),
                    vec![],
                );
            }
        } else {
            working_set.error(ParseError::NonUtf8(import_pattern.head.span));
            return (garbage_pipeline(spans), vec![]);
        }
    };

    let decls_to_use = if import_pattern.members.is_empty() {
        module.decls_with_head(&import_pattern.head.name)
    } else {
        match &import_pattern.members[0] {
            ImportPatternMember::Glob { .. } => module.decls(),
            ImportPatternMember::Name { name, span } => {
                let mut decl_output = vec![];

                if name == b"main" {
                    if let Some(id) = &module.main {
                        decl_output.push((import_pattern.head.name.clone(), *id));
                    } else {
                        working_set.error(ParseError::ExportNotFound(*span));
                    }
                } else if let Some(id) = module.get_decl_id(name) {
                    decl_output.push((name.clone(), id));
                } else {
                    working_set.error(ParseError::ExportNotFound(*span));
                }

                decl_output
            }
            ImportPatternMember::List { names } => {
                let mut decl_output = vec![];

                for (name, span) in names {
                    if name == b"main" {
                        if let Some(id) = &module.main {
                            decl_output.push((import_pattern.head.name.clone(), *id));
                        } else {
                            working_set.error(ParseError::ExportNotFound(*span));
                        }
                    } else if let Some(id) = module.get_decl_id(name) {
                        decl_output.push((name.clone(), id));
                    } else {
                        working_set.error(ParseError::ExportNotFound(*span));
                        break;
                    }
                }

                decl_output
            }
        }
    };

    let exportables = decls_to_use
        .iter()
        .map(|(name, decl_id)| Exportable::Decl {
            name: name.clone(),
            id: *decl_id,
        })
        .collect();

    // Extend the current scope with the module's exportables
    working_set.use_decls(decls_to_use);

    // Create a new Use command call to pass the new import pattern
    let import_pattern_expr = Expression {
        expr: Expr::ImportPattern(import_pattern),
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

pub fn parse_hide(working_set: &mut StateWorkingSet, spans: &[Span]) -> Pipeline {
    if working_set.get_span_contents(spans[0]) != b"hide" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for 'hide' command".into(),
            span(spans),
        ));
        return garbage_pipeline(spans);
    }

    let (call, args_spans) = match working_set.find_decl(b"hide", &Type::Any) {
        Some(decl_id) => {
            let ParsedInternalCall { call, output } =
                parse_internal_call(working_set, spans[0], &spans[1..], decl_id);
            let decl = working_set.get_decl(decl_id);

            let call_span = span(spans);

            let starting_error_count = working_set.parse_errors.len();
            check_call(working_set, call_span, &decl.signature(), &call);
            if starting_error_count != working_set.parse_errors.len() || call.has_flag("help") {
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
                if let Some(id) = working_set.find_decl(&import_pattern.head.name, &Type::Any) {
                    // a custom command,
                    let mut module = Module::new(b"tmp".to_vec());
                    module.add_decl(import_pattern.head.name.clone(), id);

                    (false, module)
                } else {
                    // , or it could be an env var (handled by the engine)
                    (false, Module::new(b"tmp".to_vec()))
                }
            } else {
                working_set.error(ParseError::ModuleNotFound(spans[1]));
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
            expr: Expr::ImportPattern(import_pattern),
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
            Ok(val) => match value_as_string(val, expr.span) {
                Ok(s) => (s, expr.span),
                Err(err) => {
                    working_set.error(err);
                    return garbage_pipeline(&[call_span]);
                }
            },
            Err(err) => {
                working_set.error(err);
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

    working_set.add_overlay(overlay_name.as_bytes().to_vec(), module_id, vec![], false);

    pipeline
}

pub fn parse_overlay_use(working_set: &mut StateWorkingSet, call: Box<Call>) -> Pipeline {
    let call_span = call.span();

    let (overlay_name, overlay_name_span) = if let Some(expr) = call.positional_nth(0) {
        match eval_constant(working_set, expr) {
            Ok(val) => match value_as_string(val, expr.span) {
                Ok(s) => (s, expr.span),
                Err(err) => {
                    working_set.error(err);
                    return garbage_pipeline(&[call_span]);
                }
            },
            Err(err) => {
                working_set.error(err);
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
                Ok(val) => match value_as_string(val, new_name_expression.span) {
                    Ok(s) => Some(Spanned {
                        item: s,
                        span: new_name_expression.span,
                    }),
                    Err(err) => {
                        working_set.error(err);
                        return garbage_pipeline(&[call_span]);
                    }
                },
                Err(err) => {
                    working_set.error(err);
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

    let has_prefix = call.has_flag("prefix");
    let do_reload = call.has_flag("reload");

    let pipeline = Pipeline::from_vec(vec![Expression {
        expr: Expr::Call(call.clone()),
        span: call_span,
        ty: Type::Any,
        custom_completion: None,
    }]);

    let cwd = working_set.get_cwd();

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
            // Create a new overlay from a module
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
            } else {
                // try if the name is a file
                if let Ok(module_filename) =
                    String::from_utf8(trim_quotes(overlay_name.as_bytes()).to_vec())
                {
                    if let Some(module_path) =
                        find_in_dirs(&module_filename, working_set, &cwd, LIB_DIRS_VAR)
                    {
                        let overlay_name = if let Some(stem) = module_path.file_stem() {
                            stem.to_string_lossy().to_string()
                        } else {
                            working_set
                                .error(ParseError::ModuleOrOverlayNotFound(overlay_name_span));
                            return pipeline;
                        };

                        if let Ok(contents) = std::fs::read(&module_path) {
                            let file_id = working_set.add_file(module_filename, &contents);
                            let new_span = working_set.get_span_for_file(file_id);

                            // Change currently parsed directory
                            let prev_currently_parsed_cwd =
                                if let Some(parent) = module_path.parent() {
                                    let prev = working_set.currently_parsed_cwd.clone();

                                    working_set.currently_parsed_cwd = Some(parent.into());

                                    prev
                                } else {
                                    working_set.currently_parsed_cwd.clone()
                                };

                            let (block, module, module_comments) =
                                parse_module_block(working_set, new_span, overlay_name.as_bytes());

                            // Restore the currently parsed directory back
                            working_set.currently_parsed_cwd = prev_currently_parsed_cwd;

                            let _ = working_set.add_block(block);
                            let module_id = working_set.add_module(
                                &overlay_name,
                                module.clone(),
                                module_comments,
                            );

                            (
                                new_name.map(|spanned| spanned.item).unwrap_or(overlay_name),
                                module,
                                module_id,
                                true,
                            )
                        } else {
                            working_set
                                .error(ParseError::ModuleOrOverlayNotFound(overlay_name_span));
                            return pipeline;
                        }
                    } else {
                        working_set.error(ParseError::ModuleOrOverlayNotFound(overlay_name_span));
                        return pipeline;
                    }
                } else {
                    working_set.error(ParseError::NonUtf8(overlay_name_span));
                    return garbage_pipeline(&[call_span]);
                }
            }
        };

    let decls_to_lay = if is_module_updated {
        if has_prefix {
            origin_module.decls_with_head(final_overlay_name.as_bytes())
        } else {
            origin_module.decls()
        }
    } else {
        vec![]
    };

    working_set.add_overlay(
        final_overlay_name.as_bytes().to_vec(),
        origin_module_id,
        decls_to_lay,
        has_prefix,
    );

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
            Ok(val) => match value_as_string(val, expr.span) {
                Ok(s) => (s, expr.span),
                Err(err) => {
                    working_set.error(err);
                    return garbage_pipeline(&[call_span]);
                }
            },
            Err(err) => {
                working_set.error(err);
                return garbage_pipeline(&[call_span]);
            }
        }
    } else {
        (
            String::from_utf8_lossy(working_set.last_overlay_name()).to_string(),
            call_span,
        )
    };

    let keep_custom = call.has_flag("keep-custom");

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
        .contains(&overlay_name.as_bytes().to_vec())
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

pub fn parse_let_or_const(working_set: &mut StateWorkingSet, spans: &[Span]) -> Pipeline {
    let name = working_set.get_span_contents(spans[0]);

    if name == b"let" || name == b"const" {
        let is_const = &name == b"const";

        if let Some(span) = check_name(working_set, spans) {
            return Pipeline::from_vec(vec![garbage(*span)]);
        }

        if let Some(decl_id) =
            working_set.find_decl(if is_const { b"const" } else { b"let" }, &Type::Any)
        {
            let cmd = working_set.get_decl(decl_id);
            let call_signature = cmd.signature().call_signature();

            if spans.len() >= 4 {
                // This is a bit of by-hand parsing to get around the issue where we want to parse in the reverse order
                // so that the var-id created by the variable isn't visible in the expression that init it
                for span in spans.iter().enumerate() {
                    let item = working_set.get_span_contents(*span.1);
                    if item == b"=" && spans.len() > (span.0 + 1) {
                        let mut idx = span.0;
                        let rvalue = parse_multispan_value(
                            working_set,
                            spans,
                            &mut idx,
                            &SyntaxShape::Keyword(
                                b"=".to_vec(),
                                Box::new(SyntaxShape::MathExpression),
                            ),
                        );

                        if idx < (spans.len() - 1) {
                            working_set
                                .error(ParseError::ExtraPositional(call_signature, spans[idx + 1]));
                        }

                        let mut idx = 0;
                        let lvalue = parse_var_with_opt_type(
                            working_set,
                            &spans[1..(span.0)],
                            &mut idx,
                            false,
                        );

                        let var_name =
                            String::from_utf8_lossy(working_set.get_span_contents(lvalue.span))
                                .trim_start_matches('$')
                                .to_string();

                        if ["in", "nu", "env", "nothing"].contains(&var_name.as_str()) {
                            working_set.error(ParseError::NameIsBuiltinVar(var_name, lvalue.span))
                        }

                        let var_id = lvalue.as_var();
                        let rhs_type = rvalue.ty.clone();

                        if let Some(var_id) = var_id {
                            working_set.set_variable_type(var_id, rhs_type);

                            if is_const {
                                match eval_constant(working_set, &rvalue) {
                                    Ok(val) => {
                                        working_set.add_constant(var_id, val);
                                    }
                                    Err(err) => working_set.error(err),
                                }
                            }
                        }

                        let call = Box::new(Call {
                            decl_id,
                            head: spans[0],
                            arguments: vec![
                                Argument::Positional(lvalue),
                                Argument::Positional(rvalue),
                            ],
                            redirect_stdout: true,
                            redirect_stderr: false,
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
        }
    }

    working_set.error(ParseError::UnknownState(
        "internal error: let or const statement unparsable".into(),
        span(spans),
    ));

    garbage_pipeline(spans)
}

pub fn parse_mut(working_set: &mut StateWorkingSet, spans: &[Span]) -> Pipeline {
    let name = working_set.get_span_contents(spans[0]);

    if name == b"mut" {
        if let Some(span) = check_name(working_set, spans) {
            return Pipeline::from_vec(vec![garbage(*span)]);
        }

        if let Some(decl_id) = working_set.find_decl(b"mut", &Type::Any) {
            let cmd = working_set.get_decl(decl_id);
            let call_signature = cmd.signature().call_signature();

            if spans.len() >= 4 {
                // This is a bit of by-hand parsing to get around the issue where we want to parse in the reverse order
                // so that the var-id created by the variable isn't visible in the expression that init it
                for span in spans.iter().enumerate() {
                    let item = working_set.get_span_contents(*span.1);
                    if item == b"=" && spans.len() > (span.0 + 1) {
                        let mut idx = span.0;
                        let rvalue = parse_multispan_value(
                            working_set,
                            spans,
                            &mut idx,
                            &SyntaxShape::Keyword(
                                b"=".to_vec(),
                                Box::new(SyntaxShape::MathExpression),
                            ),
                        );

                        if idx < (spans.len() - 1) {
                            working_set
                                .error(ParseError::ExtraPositional(call_signature, spans[idx + 1]));
                        }

                        let mut idx = 0;
                        let lvalue = parse_var_with_opt_type(
                            working_set,
                            &spans[1..(span.0)],
                            &mut idx,
                            true,
                        );

                        let var_name =
                            String::from_utf8_lossy(working_set.get_span_contents(lvalue.span))
                                .trim_start_matches('$')
                                .to_string();

                        if ["in", "nu", "env", "nothing"].contains(&var_name.as_str()) {
                            working_set.error(ParseError::NameIsBuiltinVar(var_name, lvalue.span))
                        }

                        let var_id = lvalue.as_var();
                        let rhs_type = rvalue.ty.clone();

                        if let Some(var_id) = var_id {
                            working_set.set_variable_type(var_id, rhs_type);
                        }

                        let call = Box::new(Call {
                            decl_id,
                            head: spans[0],
                            arguments: vec![
                                Argument::Positional(lvalue),
                                Argument::Positional(rvalue),
                            ],
                            redirect_stdout: true,
                            redirect_stderr: false,
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
        }
    }
    working_set.error(ParseError::UnknownState(
        "internal error: mut statement unparsable".into(),
        span(spans),
    ));

    garbage_pipeline(spans)
}

pub fn parse_source(working_set: &mut StateWorkingSet, spans: &[Span]) -> Pipeline {
    let name = working_set.get_span_contents(spans[0]);

    if name == b"source" || name == b"source-env" {
        let scoped = name == b"source-env";

        if let Some(decl_id) = working_set.find_decl(name, &Type::Any) {
            let cwd = working_set.get_cwd();

            // Is this the right call to be using here?
            // Some of the others (`parse_let`) use it, some of them (`parse_hide`) don't.
            let ParsedInternalCall { call, output } =
                parse_internal_call(working_set, spans[0], &spans[1..], decl_id);

            if call.has_flag("help") {
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
                        working_set.error(err);
                        return Pipeline::from_vec(vec![Expression {
                            expr: Expr::Call(call),
                            span: span(&spans[1..]),
                            ty: Type::Any,
                            custom_completion: None,
                        }]);
                    }
                };

                let filename = match value_as_string(val, spans[1]) {
                    Ok(s) => s,
                    Err(err) => {
                        working_set.error(err);
                        return Pipeline::from_vec(vec![Expression {
                            expr: Expr::Call(call),
                            span: span(&spans[1..]),
                            ty: Type::Any,
                            custom_completion: None,
                        }]);
                    }
                };

                if let Some(path) = find_in_dirs(&filename, working_set, &cwd, LIB_DIRS_VAR) {
                    if let Ok(contents) = std::fs::read(&path) {
                        // Change currently parsed directory
                        let prev_currently_parsed_cwd = if let Some(parent) = path.parent() {
                            let prev = working_set.currently_parsed_cwd.clone();

                            working_set.currently_parsed_cwd = Some(parent.into());

                            prev
                        } else {
                            working_set.currently_parsed_cwd.clone()
                        };

                        // This will load the defs from the file into the
                        // working set, if it was a successful parse.
                        let block = parse(
                            working_set,
                            Some(&path.to_string_lossy()),
                            &contents,
                            scoped,
                        );

                        // Restore the currently parsed directory back
                        working_set.currently_parsed_cwd = prev_currently_parsed_cwd;

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

    let call = match working_set.find_decl(b"where", &Type::Any) {
        Some(decl_id) => {
            let ParsedInternalCall { call, output } =
                parse_internal_call(working_set, spans[0], &spans[1..], decl_id);
            let decl = working_set.get_decl(decl_id);

            let call_span = span(spans);

            let starting_error_count = working_set.parse_errors.len();
            check_call(working_set, call_span, &decl.signature(), &call);
            if starting_error_count != working_set.parse_errors.len() || call.has_flag("help") {
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

pub fn parse_where(working_set: &mut StateWorkingSet, spans: &[Span]) -> Pipeline {
    let expression = parse_where_expr(working_set, spans);
    Pipeline::from_vec(vec![expression])
}

#[cfg(feature = "plugin")]
pub fn parse_register(working_set: &mut StateWorkingSet, spans: &[Span]) -> Pipeline {
    use nu_plugin::{get_signature, PluginDeclaration};
    use nu_protocol::{engine::Stack, PluginSignature};

    let cwd = working_set.get_cwd();

    // Checking that the function is used with the correct name
    // Maybe this is not necessary but it is a sanity check
    if working_set.get_span_contents(spans[0]) != b"register" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for parse plugin function".into(),
            span(spans),
        ));
        return garbage_pipeline(spans);
    }

    // Parsing the spans and checking that they match the register signature
    // Using a parsed call makes more sense than checking for how many spans are in the call
    // Also, by creating a call, it can be checked if it matches the declaration signature
    let (call, call_span) = match working_set.find_decl(b"register", &Type::Any) {
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
            if starting_error_count != working_set.parse_errors.len() || call.has_flag("help") {
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

    // Extracting the required arguments from the call and keeping them together in a tuple
    let arguments = call
        .positional_nth(0)
        .map(|expr| {
            let val = eval_constant(working_set, expr)?;
            let filename = value_as_string(val, expr.span)?;

            let Some(path) = find_in_dirs(&filename, working_set, &cwd, PLUGIN_DIRS_VAR) else {
                return Err(ParseError::RegisteredFileNotFound(filename, expr.span))
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
    let stack = Stack::new();
    let current_envs =
        nu_engine::env::env_to_strings(working_set.permanent_state, &stack).unwrap_or_default();
    let error = match signature {
        Some(signature) => arguments.and_then(|(path, path_span)| {
            // restrict plugin file name starts with `nu_plugin_`
            let valid_plugin_name = path
                .file_name()
                .map(|s| s.to_string_lossy().starts_with("nu_plugin_"));

            if let Some(true) = valid_plugin_name {
                signature.map(|signature| {
                    let plugin_decl = PluginDeclaration::new(path, signature, shell);
                    working_set.add_decl(Box::new(plugin_decl));
                    working_set.mark_plugins_file_dirty();
                })
            } else {
                Err(ParseError::LabeledError(
                    "Register plugin failed".into(),
                    "plugin name must start with nu_plugin_".into(),
                    path_span,
                ))
            }
        }),
        None => arguments.and_then(|(path, path_span)| {
            // restrict plugin file name starts with `nu_plugin_`
            let valid_plugin_name = path
                .file_name()
                .map(|s| s.to_string_lossy().starts_with("nu_plugin_"));

            if let Some(true) = valid_plugin_name {
                get_signature(path.as_path(), &shell, &current_envs)
                    .map_err(|err| {
                        ParseError::LabeledError(
                            "Error getting signatures".into(),
                            err.to_string(),
                            spans[0],
                        )
                    })
                    .map(|signatures| {
                        for signature in signatures {
                            // create plugin command declaration (need struct impl Command)
                            // store declaration in working set
                            let plugin_decl =
                                PluginDeclaration::new(path.clone(), signature, shell.clone());

                            working_set.add_decl(Box::new(plugin_decl));
                        }

                        working_set.mark_plugins_file_dirty();
                    })
            } else {
                Err(ParseError::LabeledError(
                    "Register plugin failed".into(),
                    "plugin name must start with nu_plugin_".into(),
                    path_span,
                ))
            }
        }),
    }
    .err();

    if let Some(err) = error {
        working_set.error(err);
    }

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
        .filter(|var_id| working_set.find_constant(*var_id).is_some())
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
    dirs_var_name: &str,
) -> Option<PathBuf> {
    pub fn find_in_dirs_with_id(
        filename: &str,
        working_set: &StateWorkingSet,
        cwd: &str,
        dirs_var_name: &str,
    ) -> Option<PathBuf> {
        // Choose whether to use file-relative or PWD-relative path
        let actual_cwd = if let Some(currently_parsed_cwd) = &working_set.currently_parsed_cwd {
            currently_parsed_cwd.as_path()
        } else {
            Path::new(cwd)
        };
        if let Ok(p) = canonicalize_with(filename, actual_cwd) {
            return Some(p);
        }

        let path = Path::new(filename);
        if !path.is_relative() {
            return None;
        }

        working_set
            .find_constant(find_dirs_var(working_set, dirs_var_name)?)?
            .as_list()
            .ok()?
            .iter()
            .map(|lib_dir| -> Option<PathBuf> {
                let dir = lib_dir.as_path().ok()?;
                let dir_abs = canonicalize_with(dir, actual_cwd).ok()?;
                canonicalize_with(filename, dir_abs).ok()
            })
            .find(Option::is_some)
            .flatten()
    }

    // TODO: remove (see #8310)
    pub fn find_in_dirs_old(
        filename: &str,
        working_set: &StateWorkingSet,
        cwd: &str,
        dirs_env: &str,
    ) -> Option<PathBuf> {
        // Choose whether to use file-relative or PWD-relative path
        let actual_cwd = if let Some(currently_parsed_cwd) = &working_set.currently_parsed_cwd {
            currently_parsed_cwd.as_path()
        } else {
            Path::new(cwd)
        };

        if let Ok(p) = canonicalize_with(filename, actual_cwd) {
            Some(p)
        } else {
            let path = Path::new(filename);

            if path.is_relative() {
                if let Some(lib_dirs) = working_set.get_env_var(dirs_env) {
                    if let Ok(dirs) = lib_dirs.as_list() {
                        for lib_dir in dirs {
                            if let Ok(dir) = lib_dir.as_path() {
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

    find_in_dirs_with_id(filename, working_set, cwd, dirs_var_name)
        .or_else(|| find_in_dirs_old(filename, working_set, cwd, dirs_var_name))
}
