use nu_path::canonicalize_with;
use nu_protocol::{
    ast::{
        Argument, Block, Call, Expr, Expression, ImportPattern, ImportPatternHead,
        ImportPatternMember, Pipeline,
    },
    engine::{StateWorkingSet, DEFAULT_OVERLAY_NAME},
    span, Exportable, Module, PositionalArg, Span, SyntaxShape, Type,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

static LIB_DIRS_ENV: &str = "NU_LIB_DIRS";
#[cfg(feature = "plugin")]
static PLUGIN_DIRS_ENV: &str = "NU_PLUGIN_DIRS";

use crate::{
    known_external::KnownExternal,
    lex, lite_parse,
    lite_parse::LiteCommand,
    parser::{
        check_call, check_name, garbage, garbage_pipeline, parse, parse_block_expression,
        parse_internal_call, parse_multispan_value, parse_signature, parse_string,
        parse_var_with_opt_type, trim_quotes,
    },
    unescape_unquote_string, ParseError,
};

pub fn parse_def_predecl(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    expand_aliases_denylist: &[usize],
) -> Option<ParseError> {
    let name = working_set.get_span_contents(spans[0]);

    // handle "export def" same as "def"
    let (name, spans) = if name == b"export" && spans.len() >= 2 {
        (working_set.get_span_contents(spans[1]), &spans[1..])
    } else {
        (name, spans)
    };

    if (name == b"def" || name == b"def-env") && spans.len() >= 4 {
        let (name_expr, ..) = parse_string(working_set, spans[1], expand_aliases_denylist);
        let name = name_expr.as_string();

        working_set.enter_scope();
        // FIXME: because parse_signature will update the scope with the variables it sees
        // we end up parsing the signature twice per def. The first time is during the predecl
        // so that we can see the types that are part of the signature, which we need for parsing.
        // The second time is when we actually parse the body itworking_set.
        // We can't reuse the first time because the variables that are created during parse_signature
        // are lost when we exit the scope below.
        let (sig, ..) = parse_signature(working_set, spans[2], expand_aliases_denylist);
        let signature = sig.as_signature();
        working_set.exit_scope();
        if let (Some(name), Some(mut signature)) = (name, signature) {
            signature.name = name;
            let decl = signature.predeclare();

            if working_set.add_predecl(decl).is_some() {
                return Some(ParseError::DuplicateCommandDef(spans[1]));
            }
        }
    } else if name == b"extern" && spans.len() == 3 {
        let (name_expr, ..) = parse_string(working_set, spans[1], expand_aliases_denylist);
        let name = name_expr.as_string();

        working_set.enter_scope();
        // FIXME: because parse_signature will update the scope with the variables it sees
        // we end up parsing the signature twice per def. The first time is during the predecl
        // so that we can see the types that are part of the signature, which we need for parsing.
        // The second time is when we actually parse the body itworking_set.
        // We can't reuse the first time because the variables that are created during parse_signature
        // are lost when we exit the scope below.
        let (sig, ..) = parse_signature(working_set, spans[2], expand_aliases_denylist);
        let signature = sig.as_signature();
        working_set.exit_scope();

        if let (Some(name), Some(mut signature)) = (name, signature) {
            signature.name = name.clone();
            //let decl = signature.predeclare();
            let decl = KnownExternal {
                name,
                usage: "run external command".into(),
                signature,
            };

            if working_set.add_predecl(Box::new(decl)).is_some() {
                return Some(ParseError::DuplicateCommandDef(spans[1]));
            }
        }
    }

    None
}

pub fn parse_for(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    expand_aliases_denylist: &[usize],
) -> (Expression, Option<ParseError>) {
    // Checking that the function is used with the correct name
    // Maybe this is not necessary but it is a sanity check
    if working_set.get_span_contents(spans[0]) != b"for" {
        return (
            garbage(spans[0]),
            Some(ParseError::UnknownState(
                "internal error: Wrong call name for 'for' function".into(),
                span(spans),
            )),
        );
    }

    // Parsing the spans and checking that they match the register signature
    // Using a parsed call makes more sense than checking for how many spans are in the call
    // Also, by creating a call, it can be checked if it matches the declaration signature
    let (call, call_span) = match working_set.find_decl(b"for") {
        None => {
            return (
                garbage(spans[0]),
                Some(ParseError::UnknownState(
                    "internal error: for declaration not found".into(),
                    span(spans),
                )),
            )
        }
        Some(decl_id) => {
            working_set.enter_scope();
            let (call, mut err) = parse_internal_call(
                working_set,
                spans[0],
                &spans[1..],
                decl_id,
                expand_aliases_denylist,
            );
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

            err = check_call(call_span, &sig, &call).or(err);
            if err.is_some() || call.has_flag("help") {
                return (
                    Expression {
                        expr: Expr::Call(call),
                        span: call_span,
                        ty: Type::Any,
                        custom_completion: None,
                    },
                    err,
                );
            }

            (call, call_span)
        }
    };

    // All positional arguments must be in the call positional vector by this point
    let var_decl = call.positional_nth(0).expect("for call already checked");
    let block = call.positional_nth(2).expect("for call already checked");

    let error = None;
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

    (
        Expression {
            expr: Expr::Call(call),
            span: call_span,
            ty: Type::Any,
            custom_completion: None,
        },
        error,
    )
}

fn build_usage(working_set: &StateWorkingSet, spans: &[Span]) -> String {
    let mut usage = String::new();

    let mut num_spaces = 0;
    let mut first = true;

    // Use the comments to build the usage
    for comment_part in spans {
        let contents = working_set.get_span_contents(*comment_part);

        let comment_line = if first {
            // Count the number of spaces still at the front, skipping the '#'
            let mut pos = 1;
            while pos < contents.len() {
                if let Some(b' ') = contents.get(pos) {
                    // continue
                } else {
                    break;
                }
                pos += 1;
            }

            num_spaces = pos;

            first = false;

            String::from_utf8_lossy(&contents[pos..]).to_string()
        } else {
            let mut pos = 1;

            while pos < contents.len() && pos < num_spaces {
                if let Some(b' ') = contents.get(pos) {
                    // continue
                } else {
                    break;
                }
                pos += 1;
            }

            String::from_utf8_lossy(&contents[pos..]).to_string()
        };

        if !usage.is_empty() {
            usage.push('\n');
        }
        usage.push_str(&comment_line);
    }

    usage
}

pub fn parse_def(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
    expand_aliases_denylist: &[usize],
) -> (Pipeline, Option<ParseError>) {
    let spans = &lite_command.parts[..];

    let usage = build_usage(working_set, &lite_command.comments);

    // Checking that the function is used with the correct name
    // Maybe this is not necessary but it is a sanity check

    let def_call = working_set.get_span_contents(spans[0]).to_vec();
    if def_call != b"def" && def_call != b"def-env" {
        return (
            garbage_pipeline(spans),
            Some(ParseError::UnknownState(
                "internal error: Wrong call name for def function".into(),
                span(spans),
            )),
        );
    }

    // Parsing the spans and checking that they match the register signature
    // Using a parsed call makes more sense than checking for how many spans are in the call
    // Also, by creating a call, it can be checked if it matches the declaration signature
    let (call, call_span) = match working_set.find_decl(&def_call) {
        None => {
            return (
                garbage_pipeline(spans),
                Some(ParseError::UnknownState(
                    "internal error: def declaration not found".into(),
                    span(spans),
                )),
            )
        }
        Some(decl_id) => {
            working_set.enter_scope();
            let (call, mut err) = parse_internal_call(
                working_set,
                spans[0],
                &spans[1..],
                decl_id,
                expand_aliases_denylist,
            );
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

            err = check_call(call_span, &sig, &call).or(err);
            if err.is_some() || call.has_flag("help") {
                return (
                    Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: call_span,
                        ty: Type::Any,
                        custom_completion: None,
                    }]),
                    err,
                );
            }

            (call, call_span)
        }
    };

    // All positional arguments must be in the call positional vector by this point
    let name_expr = call.positional_nth(0).expect("def call already checked");
    let sig = call.positional_nth(1).expect("def call already checked");
    let block = call.positional_nth(2).expect("def call already checked");

    let mut error = None;

    if let (Some(name), Some(mut signature), Some(block_id)) =
        (&name_expr.as_string(), sig.as_signature(), block.as_block())
    {
        if let Some(decl_id) = working_set.find_predecl(name.as_bytes()) {
            let declaration = working_set.get_decl_mut(decl_id);

            signature.name = name.clone();
            *signature = signature.add_help();
            signature.usage = usage;

            *declaration = signature.clone().into_block_command(block_id);

            let mut block = working_set.get_block_mut(block_id);
            block.signature = signature;
            block.redirect_env = def_call == b"def-env";
        } else {
            error = error.or_else(|| {
                Some(ParseError::InternalError(
                    "Predeclaration failed to add declaration".into(),
                    spans[1],
                ))
            });
        };
    }

    if let Some(name) = name_expr.as_string() {
        // It's OK if it returns None: The decl was already merged in previous parse pass.
        working_set.merge_predecl(name.as_bytes());
    } else {
        error = error.or_else(|| {
            Some(ParseError::UnknownState(
                "Could not get string from string expression".into(),
                name_expr.span,
            ))
        });
    }

    (
        Pipeline::from_vec(vec![Expression {
            expr: Expr::Call(call),
            span: call_span,
            ty: Type::Any,
            custom_completion: None,
        }]),
        error,
    )
}

pub fn parse_extern(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
    expand_aliases_denylist: &[usize],
) -> (Pipeline, Option<ParseError>) {
    let spans = &lite_command.parts[..];
    let mut error = None;

    let usage = build_usage(working_set, &lite_command.comments);

    // Checking that the function is used with the correct name
    // Maybe this is not necessary but it is a sanity check

    let extern_call = working_set.get_span_contents(spans[0]).to_vec();
    if extern_call != b"extern" {
        return (
            garbage_pipeline(spans),
            Some(ParseError::UnknownState(
                "internal error: Wrong call name for extern function".into(),
                span(spans),
            )),
        );
    }

    // Parsing the spans and checking that they match the register signature
    // Using a parsed call makes more sense than checking for how many spans are in the call
    // Also, by creating a call, it can be checked if it matches the declaration signature
    let (call, call_span) = match working_set.find_decl(&extern_call) {
        None => {
            return (
                garbage_pipeline(spans),
                Some(ParseError::UnknownState(
                    "internal error: def declaration not found".into(),
                    span(spans),
                )),
            )
        }
        Some(decl_id) => {
            working_set.enter_scope();
            let (call, err) = parse_internal_call(
                working_set,
                spans[0],
                &spans[1..],
                decl_id,
                expand_aliases_denylist,
            );
            working_set.exit_scope();

            error = error.or(err);

            let call_span = span(spans);
            //let decl = working_set.get_decl(decl_id);
            //let sig = decl.signature();

            (call, call_span)
        }
    };
    let name_expr = call.positional_nth(0);
    let sig = call.positional_nth(1);

    if let (Some(name_expr), Some(sig)) = (name_expr, sig) {
        if let (Some(name), Some(mut signature)) = (&name_expr.as_string(), sig.as_signature()) {
            if let Some(decl_id) = working_set.find_predecl(name.as_bytes()) {
                let declaration = working_set.get_decl_mut(decl_id);

                signature.name = name.clone();
                signature.usage = usage.clone();

                let decl = KnownExternal {
                    name: name.to_string(),
                    usage,
                    signature,
                };

                *declaration = Box::new(decl);
            } else {
                error = error.or_else(|| {
                    Some(ParseError::InternalError(
                        "Predeclaration failed to add declaration".into(),
                        spans[1],
                    ))
                });
            };
        }
        if let Some(name) = name_expr.as_string() {
            // It's OK if it returns None: The decl was already merged in previous parse pass.
            working_set.merge_predecl(name.as_bytes());
        } else {
            error = error.or_else(|| {
                Some(ParseError::UnknownState(
                    "Could not get string from string expression".into(),
                    name_expr.span,
                ))
            });
        }
    }

    (
        Pipeline::from_vec(vec![Expression {
            expr: Expr::Call(call),
            span: call_span,
            ty: Type::Any,
            custom_completion: None,
        }]),
        error,
    )
}

pub fn parse_alias(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    expand_aliases_denylist: &[usize],
) -> (Pipeline, Option<ParseError>) {
    let name = working_set.get_span_contents(spans[0]);

    if name == b"alias" {
        if let Some((span, err)) = check_name(working_set, spans) {
            return (Pipeline::from_vec(vec![garbage(*span)]), Some(err));
        }

        if let Some(decl_id) = working_set.find_decl(b"alias") {
            let (call, _) = parse_internal_call(
                working_set,
                spans[0],
                &spans[1..],
                decl_id,
                expand_aliases_denylist,
            );

            if call.has_flag("help") {
                return (
                    Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: span(spans),
                        ty: Type::Any,
                        custom_completion: None,
                    }]),
                    None,
                );
            }

            if spans.len() >= 4 {
                let alias_name = working_set.get_span_contents(spans[1]);

                let alias_name = if alias_name.starts_with(b"\"")
                    && alias_name.ends_with(b"\"")
                    && alias_name.len() > 1
                {
                    alias_name[1..(alias_name.len() - 1)].to_vec()
                } else {
                    alias_name.to_vec()
                };
                let _equals = working_set.get_span_contents(spans[2]);

                let replacement = spans[3..].to_vec();

                working_set.add_alias(alias_name, replacement);
            }

            let err = if spans.len() < 4 {
                Some(ParseError::IncorrectValue(
                    "Incomplete alias".into(),
                    spans[0],
                    "incomplete alias".into(),
                ))
            } else {
                None
            };

            return (
                Pipeline::from_vec(vec![Expression {
                    expr: Expr::Call(call),
                    span: span(spans),
                    ty: Type::Any,
                    custom_completion: None,
                }]),
                err,
            );
        }
    }

    (
        garbage_pipeline(spans),
        Some(ParseError::InternalError(
            "Alias statement unparseable".into(),
            span(spans),
        )),
    )
}

pub fn parse_export(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
    expand_aliases_denylist: &[usize],
) -> (Pipeline, Option<Exportable>, Option<ParseError>) {
    let spans = &lite_command.parts[..];
    let mut error = None;

    let export_span = if let Some(sp) = spans.get(0) {
        if working_set.get_span_contents(*sp) != b"export" {
            return (
                garbage_pipeline(spans),
                None,
                Some(ParseError::UnknownState(
                    "expected export statement".into(),
                    span(spans),
                )),
            );
        }

        *sp
    } else {
        return (
            garbage_pipeline(spans),
            None,
            Some(ParseError::UnknownState(
                "got empty input for parsing export statement".into(),
                span(spans),
            )),
        );
    };

    let export_decl_id = if let Some(id) = working_set.find_decl(b"export") {
        id
    } else {
        return (
            garbage_pipeline(spans),
            None,
            Some(ParseError::InternalError(
                "missing export command".into(),
                export_span,
            )),
        );
    };

    let mut call = Box::new(Call {
        head: spans[0],
        decl_id: export_decl_id,
        arguments: vec![],
        redirect_stdout: true,
        redirect_stderr: false,
    });

    let exportable = if let Some(kw_span) = spans.get(1) {
        let kw_name = working_set.get_span_contents(*kw_span);
        match kw_name {
            b"def" => {
                let lite_command = LiteCommand {
                    comments: lite_command.comments.clone(),
                    parts: spans[1..].to_vec(),
                };
                let (pipeline, err) =
                    parse_def(working_set, &lite_command, expand_aliases_denylist);
                error = error.or(err);

                let export_def_decl_id = if let Some(id) = working_set.find_decl(b"export def") {
                    id
                } else {
                    return (
                        garbage_pipeline(spans),
                        None,
                        Some(ParseError::InternalError(
                            "missing 'export def' command".into(),
                            export_span,
                        )),
                    );
                };

                // Trying to warp the 'def' call into the 'export def' in a very clumsy way
                if let Some(Expression {
                    expr: Expr::Call(ref def_call),
                    ..
                }) = pipeline.expressions.get(0)
                {
                    call = def_call.clone();

                    call.head = span(&spans[0..=1]);
                    call.decl_id = export_def_decl_id;
                } else {
                    error = error.or_else(|| {
                        Some(ParseError::InternalError(
                            "unexpected output from parsing a definition".into(),
                            span(&spans[1..]),
                        ))
                    });
                };

                if error.is_none() {
                    let decl_name = working_set.get_span_contents(spans[2]);
                    let decl_name = trim_quotes(decl_name);
                    if let Some(decl_id) = working_set.find_decl(decl_name) {
                        Some(Exportable::Decl(decl_id))
                    } else {
                        error = error.or_else(|| {
                            Some(ParseError::InternalError(
                                "failed to find added declaration".into(),
                                span(&spans[1..]),
                            ))
                        });
                        None
                    }
                } else {
                    None
                }
            }
            b"def-env" => {
                let lite_command = LiteCommand {
                    comments: lite_command.comments.clone(),
                    parts: spans[1..].to_vec(),
                };
                let (pipeline, err) =
                    parse_def(working_set, &lite_command, expand_aliases_denylist);
                error = error.or(err);

                let export_def_decl_id = if let Some(id) = working_set.find_decl(b"export def-env")
                {
                    id
                } else {
                    return (
                        garbage_pipeline(spans),
                        None,
                        Some(ParseError::InternalError(
                            "missing 'export def-env' command".into(),
                            export_span,
                        )),
                    );
                };

                // Trying to warp the 'def' call into the 'export def' in a very clumsy way
                if let Some(Expression {
                    expr: Expr::Call(ref def_call),
                    ..
                }) = pipeline.expressions.get(0)
                {
                    call = def_call.clone();

                    call.head = span(&spans[0..=1]);
                    call.decl_id = export_def_decl_id;
                } else {
                    error = error.or_else(|| {
                        Some(ParseError::InternalError(
                            "unexpected output from parsing a definition".into(),
                            span(&spans[1..]),
                        ))
                    });
                };

                if error.is_none() {
                    let decl_name = working_set.get_span_contents(spans[2]);
                    let decl_name = trim_quotes(decl_name);
                    if let Some(decl_id) = working_set.find_decl(decl_name) {
                        Some(Exportable::Decl(decl_id))
                    } else {
                        error = error.or_else(|| {
                            Some(ParseError::InternalError(
                                "failed to find added declaration".into(),
                                span(&spans[1..]),
                            ))
                        });
                        None
                    }
                } else {
                    None
                }
            }
            b"extern" => {
                let lite_command = LiteCommand {
                    comments: lite_command.comments.clone(),
                    parts: spans[1..].to_vec(),
                };
                let (pipeline, err) =
                    parse_extern(working_set, &lite_command, expand_aliases_denylist);
                error = error.or(err);

                let export_def_decl_id = if let Some(id) = working_set.find_decl(b"export extern") {
                    id
                } else {
                    return (
                        garbage_pipeline(spans),
                        None,
                        Some(ParseError::InternalError(
                            "missing 'export extern' command".into(),
                            export_span,
                        )),
                    );
                };

                // Trying to warp the 'def' call into the 'export def' in a very clumsy way
                if let Some(Expression {
                    expr: Expr::Call(ref def_call),
                    ..
                }) = pipeline.expressions.get(0)
                {
                    call = def_call.clone();

                    call.head = span(&spans[0..=1]);
                    call.decl_id = export_def_decl_id;
                } else {
                    error = error.or_else(|| {
                        Some(ParseError::InternalError(
                            "unexpected output from parsing a definition".into(),
                            span(&spans[1..]),
                        ))
                    });
                };

                if error.is_none() {
                    let decl_name = working_set.get_span_contents(spans[2]);
                    let decl_name = trim_quotes(decl_name);
                    if let Some(decl_id) = working_set.find_decl(decl_name) {
                        Some(Exportable::Decl(decl_id))
                    } else {
                        error = error.or_else(|| {
                            Some(ParseError::InternalError(
                                "failed to find added declaration".into(),
                                span(&spans[1..]),
                            ))
                        });
                        None
                    }
                } else {
                    None
                }
            }
            b"alias" => {
                let lite_command = LiteCommand {
                    comments: lite_command.comments.clone(),
                    parts: spans[1..].to_vec(),
                };
                let (pipeline, err) =
                    parse_alias(working_set, &lite_command.parts, expand_aliases_denylist);
                error = error.or(err);

                let export_alias_decl_id = if let Some(id) = working_set.find_decl(b"export alias")
                {
                    id
                } else {
                    return (
                        garbage_pipeline(spans),
                        None,
                        Some(ParseError::InternalError(
                            "missing 'export alias' command".into(),
                            export_span,
                        )),
                    );
                };

                // Trying to warp the 'alias' call into the 'export alias' in a very clumsy way
                if let Some(Expression {
                    expr: Expr::Call(ref alias_call),
                    ..
                }) = pipeline.expressions.get(0)
                {
                    call = alias_call.clone();

                    call.head = span(&spans[0..=1]);
                    call.decl_id = export_alias_decl_id;
                } else {
                    error = error.or_else(|| {
                        Some(ParseError::InternalError(
                            "unexpected output from parsing a definition".into(),
                            span(&spans[1..]),
                        ))
                    });
                };

                if error.is_none() {
                    let alias_name = working_set.get_span_contents(spans[2]);
                    let alias_name = trim_quotes(alias_name);
                    if let Some(alias_id) = working_set.find_alias(alias_name) {
                        Some(Exportable::Alias(alias_id))
                    } else {
                        error = error.or_else(|| {
                            Some(ParseError::InternalError(
                                "failed to find added alias".into(),
                                span(&spans[1..]),
                            ))
                        });
                        None
                    }
                } else {
                    None
                }
            }
            b"env" => {
                if let Some(id) = working_set.find_decl(b"export env") {
                    call.decl_id = id;
                } else {
                    return (
                        garbage_pipeline(spans),
                        None,
                        Some(ParseError::InternalError(
                            "missing 'export env' command".into(),
                            export_span,
                        )),
                    );
                }

                let sig = working_set.get_decl(call.decl_id);
                let call_signature = sig.signature().call_signature();

                call.head = span(&spans[0..=1]);

                if let Some(name_span) = spans.get(2) {
                    let (name_expr, err) =
                        parse_string(working_set, *name_span, expand_aliases_denylist);
                    error = error.or(err);
                    call.add_positional(name_expr);

                    if let Some(block_span) = spans.get(3) {
                        let (block_expr, err) = parse_block_expression(
                            working_set,
                            &SyntaxShape::Block(None),
                            *block_span,
                            expand_aliases_denylist,
                        );
                        error = error.or(err);

                        let exportable = if let Expression {
                            expr: Expr::Block(block_id),
                            ..
                        } = block_expr
                        {
                            Some(Exportable::EnvVar(block_id))
                        } else {
                            error = error.or_else(|| {
                                Some(ParseError::InternalError(
                                    "block was not parsed as a block".into(),
                                    *block_span,
                                ))
                            });
                            None
                        };

                        call.add_positional(block_expr);

                        exportable
                    } else {
                        let err_span = Span {
                            start: name_span.end,
                            end: name_span.end,
                        };

                        error = error.or_else(|| {
                            Some(ParseError::MissingPositional(
                                "block".into(),
                                err_span,
                                call_signature,
                            ))
                        });

                        None
                    }
                } else {
                    let err_span = Span {
                        start: kw_span.end,
                        end: kw_span.end,
                    };

                    error = error.or_else(|| {
                        Some(ParseError::MissingPositional(
                            "environment variable name".into(),
                            err_span,
                            call_signature,
                        ))
                    });

                    None
                }
            }
            _ => {
                error = error.or_else(|| {
                    Some(ParseError::Expected(
                        // TODO: Fill in more keywords as they come
                        "def, def-env, alias, or env keyword".into(),
                        spans[1],
                    ))
                });

                None
            }
        }
    } else {
        error = error.or_else(|| {
            Some(ParseError::MissingPositional(
                "def, def-env, alias, or env keyword".into(), // TODO: keep filling more keywords as they come
                Span {
                    start: export_span.end,
                    end: export_span.end,
                },
                "'def', `def-env`, `alias`, or 'env' keyword.".to_string(),
            ))
        });

        None
    };

    (
        Pipeline::from_vec(vec![Expression {
            expr: Expr::Call(call),
            span: span(spans),
            ty: Type::Any,
            custom_completion: None,
        }]),
        exportable,
        error,
    )
}

pub fn parse_module_block(
    working_set: &mut StateWorkingSet,
    span: Span,
    expand_aliases_denylist: &[usize],
) -> (Block, Module, Option<ParseError>) {
    let mut error = None;

    working_set.enter_scope();

    let source = working_set.get_span_contents(span);

    let (output, err) = lex(source, span.start, &[], &[], false);
    error = error.or(err);

    let (output, err) = lite_parse(&output);
    error = error.or(err);

    for pipeline in &output.block {
        // TODO: Should we add export env predecls as well?
        if pipeline.commands.len() == 1 {
            parse_def_predecl(
                working_set,
                &pipeline.commands[0].parts,
                expand_aliases_denylist,
            );
        }
    }

    let mut module = Module::from_span(span);

    let block: Block = output
        .block
        .iter()
        .map(|pipeline| {
            if pipeline.commands.len() == 1 {
                let name = working_set.get_span_contents(pipeline.commands[0].parts[0]);

                let (pipeline, err) = match name {
                    b"def" | b"def-env" => {
                        let (pipeline, err) =
                            parse_def(working_set, &pipeline.commands[0], expand_aliases_denylist);

                        (pipeline, err)
                    }
                    b"extern" => {
                        let (pipeline, err) = parse_extern(
                            working_set,
                            &pipeline.commands[0],
                            expand_aliases_denylist,
                        );

                        (pipeline, err)
                    }
                    b"alias" => {
                        let (pipeline, err) = parse_alias(
                            working_set,
                            &pipeline.commands[0].parts,
                            expand_aliases_denylist,
                        );

                        (pipeline, err)
                    }
                    // TODO: Currently, it is not possible to define a private env var.
                    // TODO: Exported env vars are usable iside the module only if correctly
                    // exported by the user. For example:
                    //
                    //   > module foo { export env a { "2" }; export def b [] { $env.a } }
                    //
                    // will work only if you call `use foo *; b` but not with `use foo; foo b`
                    // since in the second case, the name of the env var would be $env."foo a".
                    b"export" => {
                        let (pipe, exportable, err) = parse_export(
                            working_set,
                            &pipeline.commands[0],
                            expand_aliases_denylist,
                        );

                        if err.is_none() {
                            let name_span = pipeline.commands[0].parts[2];
                            let name = working_set.get_span_contents(name_span);
                            let name = trim_quotes(name);

                            match exportable {
                                Some(Exportable::Decl(decl_id)) => {
                                    module.add_decl(name, decl_id);
                                }
                                Some(Exportable::EnvVar(block_id)) => {
                                    module.add_env_var(name, block_id);
                                }
                                Some(Exportable::Alias(alias_id)) => {
                                    module.add_alias(name, alias_id);
                                }
                                None => {} // None should always come with error from parse_export()
                            }
                        }

                        (pipe, err)
                    }
                    _ => (
                        garbage_pipeline(&pipeline.commands[0].parts),
                        Some(ParseError::ExpectedKeyword(
                            "def or export keyword".into(),
                            pipeline.commands[0].parts[0],
                        )),
                    ),
                };

                if error.is_none() {
                    error = err;
                }

                pipeline
            } else {
                error = Some(ParseError::Expected("not a pipeline".into(), span));
                garbage_pipeline(&[span])
            }
        })
        .into();

    working_set.exit_scope();

    (block, module, error)
}

pub fn parse_module(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    expand_aliases_denylist: &[usize],
) -> (Pipeline, Option<ParseError>) {
    // TODO: Currently, module is closing over its parent scope (i.e., defs in the parent scope are
    // visible and usable in this module's scope). We want to disable that for files.

    let mut error = None;
    let bytes = working_set.get_span_contents(spans[0]);

    if bytes == b"module" && spans.len() >= 3 {
        let (module_name_expr, err) = parse_string(working_set, spans[1], expand_aliases_denylist);
        error = error.or(err);

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
            return (
                garbage_pipeline(spans),
                Some(ParseError::Expected("block".into(), block_span)),
            );
        }

        if block_bytes.ends_with(b"}") {
            end -= 1;
        } else {
            error =
                error.or_else(|| Some(ParseError::Unclosed("}".into(), Span { start: end, end })));
        }

        let block_span = Span { start, end };

        let (block, module, err) =
            parse_module_block(working_set, block_span, expand_aliases_denylist);
        error = error.or(err);

        let block_id = working_set.add_block(block);
        let _ = working_set.add_module(&module_name, module);

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
            head: spans[0],
            decl_id: module_decl_id,
            arguments: vec![
                Argument::Positional(module_name_expr),
                Argument::Positional(block_expr),
            ],
            redirect_stdout: true,
            redirect_stderr: false,
        });

        (
            Pipeline::from_vec(vec![Expression {
                expr: Expr::Call(call),
                span: span(spans),
                ty: Type::Any,
                custom_completion: None,
            }]),
            error,
        )
    } else {
        (
            garbage_pipeline(spans),
            Some(ParseError::UnknownState(
                "Expected structure: module <name> {}".into(),
                span(spans),
            )),
        )
    }
}

pub fn parse_use(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    expand_aliases_denylist: &[usize],
) -> (Pipeline, Option<ParseError>) {
    if working_set.get_span_contents(spans[0]) != b"use" {
        return (
            garbage_pipeline(spans),
            Some(ParseError::UnknownState(
                "internal error: Wrong call name for 'use' command".into(),
                span(spans),
            )),
        );
    }

    let (call, call_span, use_decl_id) = match working_set.find_decl(b"use") {
        Some(decl_id) => {
            let (call, mut err) = parse_internal_call(
                working_set,
                spans[0],
                &spans[1..],
                decl_id,
                expand_aliases_denylist,
            );
            let decl = working_set.get_decl(decl_id);

            let call_span = span(spans);

            err = check_call(call_span, &decl.signature(), &call).or(err);
            if err.is_some() || call.has_flag("help") {
                return (
                    Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: call_span,
                        ty: Type::Any,
                        custom_completion: None,
                    }]),
                    err,
                );
            }

            (call, call_span, decl_id)
        }
        None => {
            return (
                garbage_pipeline(spans),
                Some(ParseError::UnknownState(
                    "internal error: 'use' declaration not found".into(),
                    span(spans),
                )),
            )
        }
    };

    let import_pattern = if let Some(expr) = call.positional_nth(0) {
        if let Some(pattern) = expr.as_import_pattern() {
            pattern
        } else {
            return (
                garbage_pipeline(spans),
                Some(ParseError::UnknownState(
                    "internal error: Import pattern positional is not import pattern".into(),
                    expr.span,
                )),
            );
        }
    } else {
        return (
            garbage_pipeline(spans),
            Some(ParseError::UnknownState(
                "internal error: Missing required positional after call parsing".into(),
                call_span,
            )),
        );
    };

    let cwd = working_set.get_cwd();

    let mut error = None;

    // TODO: Add checking for importing too long import patterns, e.g.:
    // > use spam foo non existent names here do not throw error
    let (import_pattern, module) =
        if let Some(module_id) = working_set.find_module(&import_pattern.head.name) {
            (import_pattern, working_set.get_module(module_id).clone())
        } else {
            // TODO: Do not close over when loading module from file?
            // It could be a file

            let (module_filename, err) =
                unescape_unquote_string(&import_pattern.head.name, import_pattern.head.span);
            if err.is_none() {
                if let Some(module_path) =
                    find_in_dirs(&module_filename, working_set, &cwd, LIB_DIRS_ENV)
                {
                    let module_name = if let Some(stem) = module_path.file_stem() {
                        stem.to_string_lossy().to_string()
                    } else {
                        return (
                            Pipeline::from_vec(vec![Expression {
                                expr: Expr::Call(call),
                                span: call_span,
                                ty: Type::Any,
                                custom_completion: None,
                            }]),
                            Some(ParseError::ModuleNotFound(spans[1])),
                        );
                    };

                    if let Ok(contents) = std::fs::read(module_path) {
                        let span_start = working_set.next_span_start();
                        working_set.add_file(module_filename, &contents);
                        let span_end = working_set.next_span_start();

                        let (block, module, err) = parse_module_block(
                            working_set,
                            Span::new(span_start, span_end),
                            expand_aliases_denylist,
                        );
                        error = error.or(err);

                        let _ = working_set.add_block(block);
                        let module_id = working_set.add_module(&module_name, module.clone());

                        (
                            ImportPattern {
                                head: ImportPatternHead {
                                    name: module_name.into(),
                                    id: Some(module_id),
                                    span: spans[1],
                                },
                                members: import_pattern.members,
                                hidden: HashSet::new(),
                            },
                            module,
                        )
                    } else {
                        return (
                            Pipeline::from_vec(vec![Expression {
                                expr: Expr::Call(call),
                                span: call_span,
                                ty: Type::Any,
                                custom_completion: None,
                            }]),
                            Some(ParseError::ModuleNotFound(spans[1])),
                        );
                    }
                } else {
                    error = error.or(Some(ParseError::ModuleNotFound(import_pattern.head.span)));

                    let mut import_pattern = ImportPattern::new();
                    import_pattern.head.span = spans[1];

                    (import_pattern, Module::new())
                }
            } else {
                return (garbage_pipeline(spans), Some(ParseError::NonUtf8(spans[1])));
            }
        };

    let (decls_to_use, aliases_to_use) = if import_pattern.members.is_empty() {
        (
            module.decls_with_head(&import_pattern.head.name),
            module.aliases_with_head(&import_pattern.head.name),
        )
    } else {
        match &import_pattern.members[0] {
            ImportPatternMember::Glob { .. } => (module.decls(), module.aliases()),
            ImportPatternMember::Name { name, span } => {
                let mut decl_output = vec![];
                let mut alias_output = vec![];

                if let Some(id) = module.get_decl_id(name) {
                    decl_output.push((name.clone(), id));
                } else if let Some(id) = module.get_alias_id(name) {
                    alias_output.push((name.clone(), id));
                } else if !module.has_env_var(name) {
                    error = error.or(Some(ParseError::ExportNotFound(*span)))
                }

                (decl_output, alias_output)
            }
            ImportPatternMember::List { names } => {
                let mut decl_output = vec![];
                let mut alias_output = vec![];

                for (name, span) in names {
                    if let Some(id) = module.get_decl_id(name) {
                        decl_output.push((name.clone(), id));
                    } else if let Some(id) = module.get_alias_id(name) {
                        alias_output.push((name.clone(), id));
                    } else if !module.has_env_var(name) {
                        error = error.or(Some(ParseError::ExportNotFound(*span)));
                        break;
                    }
                }

                (decl_output, alias_output)
            }
        }
    };

    // Extend the current scope with the module's exportables
    working_set.use_decls(decls_to_use);
    working_set.use_aliases(aliases_to_use);

    // Create a new Use command call to pass the new import pattern
    let import_pattern_expr = Expression {
        expr: Expr::ImportPattern(import_pattern),
        span: span(&spans[1..]),
        ty: Type::List(Box::new(Type::String)),
        custom_completion: None,
    };

    let call = Box::new(Call {
        head: spans[0],
        decl_id: use_decl_id,
        arguments: vec![Argument::Positional(import_pattern_expr)],
        redirect_stdout: true,
        redirect_stderr: false,
    });

    (
        Pipeline::from_vec(vec![Expression {
            expr: Expr::Call(call),
            span: span(spans),
            ty: Type::Any,
            custom_completion: None,
        }]),
        error,
    )
}

pub fn parse_hide(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    expand_aliases_denylist: &[usize],
) -> (Pipeline, Option<ParseError>) {
    if working_set.get_span_contents(spans[0]) != b"hide" {
        return (
            garbage_pipeline(spans),
            Some(ParseError::UnknownState(
                "internal error: Wrong call name for 'hide' command".into(),
                span(spans),
            )),
        );
    }

    let (call, call_span, hide_decl_id) = match working_set.find_decl(b"hide") {
        Some(decl_id) => {
            let (call, mut err) = parse_internal_call(
                working_set,
                spans[0],
                &spans[1..],
                decl_id,
                expand_aliases_denylist,
            );
            let decl = working_set.get_decl(decl_id);

            let call_span = span(spans);

            err = check_call(call_span, &decl.signature(), &call).or(err);
            if err.is_some() || call.has_flag("help") {
                return (
                    Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: call_span,
                        ty: Type::Any,
                        custom_completion: None,
                    }]),
                    err,
                );
            }

            (call, call_span, decl_id)
        }
        None => {
            return (
                garbage_pipeline(spans),
                Some(ParseError::UnknownState(
                    "internal error: 'hide' declaration not found".into(),
                    span(spans),
                )),
            )
        }
    };

    let import_pattern = if let Some(expr) = call.positional_nth(0) {
        if let Some(pattern) = expr.as_import_pattern() {
            pattern
        } else {
            return (
                garbage_pipeline(spans),
                Some(ParseError::UnknownState(
                    "internal error: Import pattern positional is not import pattern".into(),
                    call_span,
                )),
            );
        }
    } else {
        return (
            garbage_pipeline(spans),
            Some(ParseError::UnknownState(
                "internal error: Missing required positional after call parsing".into(),
                call_span,
            )),
        );
    };

    let mut error = None;
    let bytes = working_set.get_span_contents(spans[0]);

    if bytes == b"hide" && spans.len() >= 2 {
        for span in spans[1..].iter() {
            let (_, err) = parse_string(working_set, *span, expand_aliases_denylist);
            error = error.or(err);
        }

        let (is_module, module) =
            if let Some(module_id) = working_set.find_module(&import_pattern.head.name) {
                (true, working_set.get_module(module_id).clone())
            } else if import_pattern.members.is_empty() {
                // The pattern head can be:
                if let Some(id) = working_set.find_alias(&import_pattern.head.name) {
                    // an alias,
                    let mut module = Module::new();
                    module.add_alias(&import_pattern.head.name, id);

                    (false, module)
                } else if let Some(id) = working_set.find_decl(&import_pattern.head.name) {
                    // a custom command,
                    let mut module = Module::new();
                    module.add_decl(&import_pattern.head.name, id);

                    (false, module)
                } else {
                    // , or it could be an env var (handled by the engine)
                    (false, Module::new())
                }
            } else {
                return (
                    garbage_pipeline(spans),
                    Some(ParseError::ModuleNotFound(spans[1])),
                );
            };

        // This kind of inverts the import pattern matching found in parse_use()
        let (aliases_to_hide, decls_to_hide) = if import_pattern.members.is_empty() {
            if is_module {
                (
                    module.alias_names_with_head(&import_pattern.head.name),
                    module.decl_names_with_head(&import_pattern.head.name),
                )
            } else {
                (module.alias_names(), module.decl_names())
            }
        } else {
            match &import_pattern.members[0] {
                ImportPatternMember::Glob { .. } => (module.alias_names(), module.decl_names()),
                ImportPatternMember::Name { name, span } => {
                    let mut aliases = vec![];
                    let mut decls = vec![];

                    if let Some(item) = module.alias_name_with_head(name, &import_pattern.head.name)
                    {
                        aliases.push(item);
                    } else if let Some(item) =
                        module.decl_name_with_head(name, &import_pattern.head.name)
                    {
                        decls.push(item);
                    } else if !module.has_env_var(name) {
                        error = error.or(Some(ParseError::ExportNotFound(*span)));
                    }

                    (aliases, decls)
                }
                ImportPatternMember::List { names } => {
                    let mut aliases = vec![];
                    let mut decls = vec![];

                    for (name, span) in names {
                        if let Some(item) =
                            module.alias_name_with_head(name, &import_pattern.head.name)
                        {
                            aliases.push(item);
                        } else if let Some(item) =
                            module.decl_name_with_head(name, &import_pattern.head.name)
                        {
                            decls.push(item);
                        } else if !module.has_env_var(name) {
                            error = error.or(Some(ParseError::ExportNotFound(*span)));
                            break;
                        }
                    }

                    (aliases, decls)
                }
            }
        };

        let import_pattern = {
            let aliases: HashSet<Vec<u8>> = aliases_to_hide.iter().cloned().collect();
            let decls: HashSet<Vec<u8>> = decls_to_hide.iter().cloned().collect();

            import_pattern.with_hidden(decls.union(&aliases).cloned().collect())
        };

        // TODO: `use spam; use spam foo; hide foo` will hide both `foo` and `spam foo` since
        // they point to the same DeclId. Do we want to keep it that way?
        working_set.hide_decls(&decls_to_hide);
        working_set.hide_aliases(&aliases_to_hide);

        // Create a new Use command call to pass the new import pattern
        let import_pattern_expr = Expression {
            expr: Expr::ImportPattern(import_pattern),
            span: span(&spans[1..]),
            ty: Type::List(Box::new(Type::String)),
            custom_completion: None,
        };

        let call = Box::new(Call {
            head: spans[0],
            decl_id: hide_decl_id,
            arguments: vec![Argument::Positional(import_pattern_expr)],
            redirect_stdout: true,
            redirect_stderr: false,
        });

        (
            Pipeline::from_vec(vec![Expression {
                expr: Expr::Call(call),
                span: span(spans),
                ty: Type::Any,
                custom_completion: None,
            }]),
            error,
        )
    } else {
        (
            garbage_pipeline(spans),
            Some(ParseError::UnknownState(
                "Expected structure: hide <name>".into(),
                span(spans),
            )),
        )
    }
}

pub fn parse_overlay(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    expand_aliases_denylist: &[usize],
) -> (Pipeline, Option<ParseError>) {
    if working_set.get_span_contents(spans[0]) != b"overlay" {
        return (
            garbage_pipeline(spans),
            Some(ParseError::UnknownState(
                "internal error: Wrong call name for 'overlay' command".into(),
                span(spans),
            )),
        );
    }

    if spans.len() > 1 {
        let subcommand = working_set.get_span_contents(spans[1]);

        match subcommand {
            b"add" => {
                return parse_overlay_add(working_set, spans, expand_aliases_denylist);
            }
            b"list" => {
                // TODO: Abstract this code blob, it's repeated all over the place:
                let call = match working_set.find_decl(b"overlay list") {
                    Some(decl_id) => {
                        let (call, mut err) = parse_internal_call(
                            working_set,
                            span(&spans[..2]),
                            if spans.len() > 2 { &spans[2..] } else { &[] },
                            decl_id,
                            expand_aliases_denylist,
                        );
                        let decl = working_set.get_decl(decl_id);

                        let call_span = span(spans);

                        err = check_call(call_span, &decl.signature(), &call).or(err);
                        if err.is_some() || call.has_flag("help") {
                            return (
                                Pipeline::from_vec(vec![Expression {
                                    expr: Expr::Call(call),
                                    span: call_span,
                                    ty: Type::Any,
                                    custom_completion: None,
                                }]),
                                err,
                            );
                        }

                        call
                    }
                    None => {
                        return (
                            garbage_pipeline(spans),
                            Some(ParseError::UnknownState(
                                "internal error: 'overlay' declaration not found".into(),
                                span(spans),
                            )),
                        )
                    }
                };

                return (
                    Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: span(spans),
                        ty: Type::Any,
                        custom_completion: None,
                    }]),
                    None,
                );
            }
            b"new" => {
                return parse_overlay_new(working_set, spans, expand_aliases_denylist);
            }
            b"remove" => {
                return parse_overlay_remove(working_set, spans, expand_aliases_denylist);
            }
            _ => { /* continue parsing overlay */ }
        }
    }

    let call = match working_set.find_decl(b"overlay") {
        Some(decl_id) => {
            let (call, mut err) = parse_internal_call(
                working_set,
                spans[0],
                &spans[1..],
                decl_id,
                expand_aliases_denylist,
            );
            let decl = working_set.get_decl(decl_id);

            let call_span = span(spans);

            err = check_call(call_span, &decl.signature(), &call).or(err);
            if err.is_some() || call.has_flag("help") {
                return (
                    Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: call_span,
                        ty: Type::Any,
                        custom_completion: None,
                    }]),
                    err,
                );
            }

            call
        }
        None => {
            return (
                garbage_pipeline(spans),
                Some(ParseError::UnknownState(
                    "internal error: 'overlay' declaration not found".into(),
                    span(spans),
                )),
            )
        }
    };

    (
        Pipeline::from_vec(vec![Expression {
            expr: Expr::Call(call),
            span: span(spans),
            ty: Type::Any,
            custom_completion: None,
        }]),
        None,
    )
}

pub fn parse_overlay_new(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    expand_aliases_denylist: &[usize],
) -> (Pipeline, Option<ParseError>) {
    if spans.len() > 1 && working_set.get_span_contents(span(&spans[0..2])) != b"overlay new" {
        return (
            garbage_pipeline(spans),
            Some(ParseError::UnknownState(
                "internal error: Wrong call name for 'overlay new' command".into(),
                span(spans),
            )),
        );
    }

    let (call, call_span) = match working_set.find_decl(b"overlay new") {
        Some(decl_id) => {
            let (call, mut err) = parse_internal_call(
                working_set,
                span(&spans[0..2]),
                &spans[2..],
                decl_id,
                expand_aliases_denylist,
            );
            let decl = working_set.get_decl(decl_id);

            let call_span = span(spans);

            err = check_call(call_span, &decl.signature(), &call).or(err);
            if err.is_some() || call.has_flag("help") {
                return (
                    Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: call_span,
                        ty: Type::Any,
                        custom_completion: None,
                    }]),
                    err,
                );
            }

            (call, call_span)
        }
        None => {
            return (
                garbage_pipeline(spans),
                Some(ParseError::UnknownState(
                    "internal error: 'overlay new' declaration not found".into(),
                    span(spans),
                )),
            )
        }
    };

    let (overlay_name, _) = if let Some(expr) = call.positional_nth(0) {
        if let Some(s) = expr.as_string() {
            (s, expr.span)
        } else {
            return (
                garbage_pipeline(spans),
                Some(ParseError::UnknownState(
                    "internal error: Module name not a string".into(),
                    expr.span,
                )),
            );
        }
    } else {
        return (
            garbage_pipeline(spans),
            Some(ParseError::UnknownState(
                "internal error: Missing required positional after call parsing".into(),
                call_span,
            )),
        );
    };

    let pipeline = Pipeline::from_vec(vec![Expression {
        expr: Expr::Call(call),
        span: span(spans),
        ty: Type::Any,
        custom_completion: None,
    }]);

    let module_id = working_set.add_module(&overlay_name, Module::new());

    working_set.add_overlay(overlay_name.as_bytes().to_vec(), module_id, vec![], vec![]);

    (pipeline, None)
}

pub fn parse_overlay_add(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    expand_aliases_denylist: &[usize],
) -> (Pipeline, Option<ParseError>) {
    if spans.len() > 1 && working_set.get_span_contents(span(&spans[0..2])) != b"overlay add" {
        return (
            garbage_pipeline(spans),
            Some(ParseError::UnknownState(
                "internal error: Wrong call name for 'overlay add' command".into(),
                span(spans),
            )),
        );
    }

    // TODO: Allow full import pattern as argument (requires custom naming of module/overlay)
    let (call, call_span) = match working_set.find_decl(b"overlay add") {
        Some(decl_id) => {
            let (call, mut err) = parse_internal_call(
                working_set,
                span(&spans[0..2]),
                &spans[2..],
                decl_id,
                expand_aliases_denylist,
            );
            let decl = working_set.get_decl(decl_id);

            let call_span = span(spans);

            err = check_call(call_span, &decl.signature(), &call).or(err);
            if err.is_some() || call.has_flag("help") {
                return (
                    Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: call_span,
                        ty: Type::Any,
                        custom_completion: None,
                    }]),
                    err,
                );
            }

            (call, call_span)
        }
        None => {
            return (
                garbage_pipeline(spans),
                Some(ParseError::UnknownState(
                    "internal error: 'overlay add' declaration not found".into(),
                    span(spans),
                )),
            )
        }
    };

    let (overlay_name, overlay_name_span) = if let Some(expr) = call.positional_nth(0) {
        if let Some(s) = expr.as_string() {
            (s, expr.span)
        } else {
            return (
                garbage_pipeline(spans),
                Some(ParseError::UnknownState(
                    "internal error: Module name not a string".into(),
                    expr.span,
                )),
            );
        }
    } else {
        return (
            garbage_pipeline(spans),
            Some(ParseError::UnknownState(
                "internal error: Missing required positional after call parsing".into(),
                call_span,
            )),
        );
    };

    let pipeline = Pipeline::from_vec(vec![Expression {
        expr: Expr::Call(call),
        span: span(spans),
        ty: Type::Any,
        custom_completion: None,
    }]);

    // TODO: Add support for it -- needs to play well with overlay remove
    let has_prefix = false; //call.has_flag("prefix");

    let cwd = working_set.get_cwd();

    let mut error = None;

    let result = if let Some(module_id) = working_set.find_overlay_origin(overlay_name.as_bytes()) {
        // Activate existing overlay
        if let Some(new_module_id) = working_set.find_module(overlay_name.as_bytes()) {
            if module_id == new_module_id {
                Some((overlay_name, Module::new(), module_id))
            } else {
                // The origin module of an overlay changed => update it
                Some((
                    overlay_name,
                    working_set.get_module(new_module_id).clone(),
                    new_module_id,
                ))
            }
        } else {
            Some((overlay_name, Module::new(), module_id))
        }
    } else {
        // Create a new overlay from a module
        if let Some(module_id) =
            // the name is a module
            working_set.find_module(overlay_name.as_bytes())
        {
            Some((
                overlay_name,
                working_set.get_module(module_id).clone(),
                module_id,
            ))
        } else {
            // try if the name is a file
            if let Ok(module_filename) =
                String::from_utf8(trim_quotes(overlay_name.as_bytes()).to_vec())
            {
                if let Some(module_path) =
                    find_in_dirs(&module_filename, working_set, &cwd, LIB_DIRS_ENV)
                {
                    let overlay_name = if let Some(stem) = module_path.file_stem() {
                        stem.to_string_lossy().to_string()
                    } else {
                        return (
                            pipeline,
                            Some(ParseError::ModuleOrOverlayNotFound(spans[1])),
                        );
                    };

                    if let Ok(contents) = std::fs::read(module_path) {
                        let span_start = working_set.next_span_start();
                        working_set.add_file(module_filename, &contents);
                        let span_end = working_set.next_span_start();

                        let (block, module, err) = parse_module_block(
                            working_set,
                            Span::new(span_start, span_end),
                            expand_aliases_denylist,
                        );
                        error = error.or(err);

                        let _ = working_set.add_block(block);
                        let module_id = working_set.add_module(&overlay_name, module.clone());

                        Some((overlay_name, module, module_id))
                    } else {
                        return (
                            pipeline,
                            Some(ParseError::ModuleOrOverlayNotFound(spans[1])),
                        );
                    }
                } else {
                    error = error.or(Some(ParseError::ModuleOrOverlayNotFound(overlay_name_span)));
                    None
                }
            } else {
                return (garbage_pipeline(spans), Some(ParseError::NonUtf8(spans[1])));
            }
        }
    };

    if let Some((name, module, module_id)) = result {
        let (decls_to_lay, aliases_to_lay) = if has_prefix {
            (
                module.decls_with_head(name.as_bytes()),
                module.aliases_with_head(name.as_bytes()),
            )
        } else {
            (module.decls(), module.aliases())
        };

        working_set.add_overlay(
            name.as_bytes().to_vec(),
            module_id,
            decls_to_lay,
            aliases_to_lay,
        );
    }

    (pipeline, error)
}

pub fn parse_overlay_remove(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    expand_aliases_denylist: &[usize],
) -> (Pipeline, Option<ParseError>) {
    if spans.len() > 1 && working_set.get_span_contents(span(&spans[0..2])) != b"overlay remove" {
        return (
            garbage_pipeline(spans),
            Some(ParseError::UnknownState(
                "internal error: Wrong call name for 'overlay remove' command".into(),
                span(spans),
            )),
        );
    }

    let call = match working_set.find_decl(b"overlay remove") {
        Some(decl_id) => {
            let (call, mut err) = parse_internal_call(
                working_set,
                span(&spans[0..2]),
                &spans[2..],
                decl_id,
                expand_aliases_denylist,
            );
            let decl = working_set.get_decl(decl_id);

            let call_span = span(spans);

            err = check_call(call_span, &decl.signature(), &call).or(err);
            if err.is_some() || call.has_flag("help") {
                return (
                    Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: call_span,
                        ty: Type::Any,
                        custom_completion: None,
                    }]),
                    err,
                );
            }

            call
        }
        None => {
            return (
                garbage_pipeline(spans),
                Some(ParseError::UnknownState(
                    "internal error: 'overlay remove' declaration not found".into(),
                    span(spans),
                )),
            )
        }
    };

    let (overlay_name, overlay_name_span) = if let Some(expr) = call.positional_nth(0) {
        if let Some(s) = expr.as_string() {
            (s, expr.span)
        } else {
            return (
                garbage_pipeline(spans),
                Some(ParseError::UnknownState(
                    "internal error: Module name not a string".into(),
                    expr.span,
                )),
            );
        }
    } else {
        (
            String::from_utf8_lossy(working_set.last_overlay_name()).to_string(),
            call.head,
        )
    };

    let keep_custom = call.has_flag("keep-custom");

    let pipeline = Pipeline::from_vec(vec![Expression {
        expr: Expr::Call(call),
        span: span(spans),
        ty: Type::Any,
        custom_completion: None,
    }]);

    if overlay_name == DEFAULT_OVERLAY_NAME {
        return (
            pipeline,
            Some(ParseError::CantRemoveDefaultOverlay(
                overlay_name,
                overlay_name_span,
            )),
        );
    }

    if !working_set
        .unique_overlay_names()
        .contains(&overlay_name.as_bytes().to_vec())
    {
        return (
            pipeline,
            Some(ParseError::ActiveOverlayNotFound(overlay_name_span)),
        );
    }

    if working_set.num_overlays() < 2 {
        return (
            pipeline,
            Some(ParseError::CantRemoveLastOverlay(overlay_name_span)),
        );
    }

    working_set.remove_overlay(overlay_name.as_bytes(), keep_custom);

    (pipeline, None)
}

pub fn parse_let(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    expand_aliases_denylist: &[usize],
) -> (Pipeline, Option<ParseError>) {
    let name = working_set.get_span_contents(spans[0]);

    if name == b"let" {
        if let Some((span, err)) = check_name(working_set, spans) {
            return (Pipeline::from_vec(vec![garbage(*span)]), Some(err));
        }

        if let Some(decl_id) = working_set.find_decl(b"let") {
            let cmd = working_set.get_decl(decl_id);
            let call_signature = cmd.signature().call_signature();

            if spans.len() >= 4 {
                // This is a bit of by-hand parsing to get around the issue where we want to parse in the reverse order
                // so that the var-id created by the variable isn't visible in the expression that init it
                for span in spans.iter().enumerate() {
                    let item = working_set.get_span_contents(*span.1);
                    if item == b"=" && spans.len() > (span.0 + 1) {
                        let mut error = None;

                        let mut idx = span.0;
                        let (rvalue, err) = parse_multispan_value(
                            working_set,
                            spans,
                            &mut idx,
                            &SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::Expression)),
                            expand_aliases_denylist,
                        );
                        error = error.or(err);

                        if idx < (spans.len() - 1) {
                            error = error.or(Some(ParseError::ExtraPositional(
                                call_signature,
                                spans[idx + 1],
                            )));
                        }

                        let mut idx = 0;
                        let (lvalue, err) =
                            parse_var_with_opt_type(working_set, &spans[1..(span.0)], &mut idx);
                        error = error.or(err);

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
                        });

                        return (
                            Pipeline::from_vec(vec![Expression {
                                expr: Expr::Call(call),
                                span: nu_protocol::span(spans),
                                ty: Type::Any,
                                custom_completion: None,
                            }]),
                            error,
                        );
                    }
                }
            }
            let (call, err) = parse_internal_call(
                working_set,
                spans[0],
                &spans[1..],
                decl_id,
                expand_aliases_denylist,
            );

            return (
                Pipeline {
                    expressions: vec![Expression {
                        expr: Expr::Call(call),
                        span: nu_protocol::span(spans),
                        ty: Type::Any,
                        custom_completion: None,
                    }],
                },
                err,
            );
        }
    }
    (
        garbage_pipeline(spans),
        Some(ParseError::UnknownState(
            "internal error: let statement unparseable".into(),
            span(spans),
        )),
    )
}

pub fn parse_source(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    expand_aliases_denylist: &[usize],
) -> (Pipeline, Option<ParseError>) {
    let mut error = None;
    let name = working_set.get_span_contents(spans[0]);

    if name == b"source" {
        if let Some(decl_id) = working_set.find_decl(b"source") {
            let cwd = working_set.get_cwd();
            // Is this the right call to be using here?
            // Some of the others (`parse_let`) use it, some of them (`parse_hide`) don't.
            let (call, err) = parse_internal_call(
                working_set,
                spans[0],
                &spans[1..],
                decl_id,
                expand_aliases_denylist,
            );
            error = error.or(err);

            if error.is_some() || call.has_flag("help") {
                return (
                    Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: span(spans),
                        ty: Type::Any,
                        custom_completion: None,
                    }]),
                    error,
                );
            }

            // Command and one file name
            if spans.len() >= 2 {
                let name_expr = working_set.get_span_contents(spans[1]);
                let (filename, err) = unescape_unquote_string(name_expr, spans[1]);
                if err.is_none() {
                    if let Some(path) = find_in_dirs(&filename, working_set, &cwd, LIB_DIRS_ENV) {
                        if let Ok(contents) = std::fs::read(&path) {
                            // This will load the defs from the file into the
                            // working set, if it was a successful parse.
                            let (block, err) = parse(
                                working_set,
                                path.file_name().and_then(|x| x.to_str()),
                                &contents,
                                false,
                                expand_aliases_denylist,
                            );

                            if err.is_some() {
                                // Unsuccessful parse of file
                                return (
                                    Pipeline::from_vec(vec![Expression {
                                        expr: Expr::Call(call),
                                        span: span(&spans[1..]),
                                        ty: Type::Any,
                                        custom_completion: None,
                                    }]),
                                    // Return the file parse error
                                    err,
                                );
                            } else {
                                // Save the block into the working set
                                let block_id = working_set.add_block(block);

                                let mut call_with_block = call;

                                // Adding this expression to the positional creates a syntax highlighting error
                                // after writing `source example.nu`
                                call_with_block.add_positional(Expression {
                                    expr: Expr::Int(block_id as i64),
                                    span: spans[1],
                                    ty: Type::Any,
                                    custom_completion: None,
                                });

                                return (
                                    Pipeline::from_vec(vec![Expression {
                                        expr: Expr::Call(call_with_block),
                                        span: span(spans),
                                        ty: Type::Any,
                                        custom_completion: None,
                                    }]),
                                    None,
                                );
                            }
                        }
                    } else {
                        error = error.or(Some(ParseError::SourcedFileNotFound(filename, spans[1])));
                    }
                } else {
                    return (garbage_pipeline(spans), Some(ParseError::NonUtf8(spans[1])));
                }
            }
            return (
                Pipeline::from_vec(vec![Expression {
                    expr: Expr::Call(call),
                    span: span(spans),
                    ty: Type::Any,
                    custom_completion: None,
                }]),
                error,
            );
        }
    }
    (
        garbage_pipeline(spans),
        Some(ParseError::UnknownState(
            "internal error: source statement unparseable".into(),
            span(spans),
        )),
    )
}

#[cfg(feature = "plugin")]
pub fn parse_register(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    expand_aliases_denylist: &[usize],
) -> (Pipeline, Option<ParseError>) {
    use nu_plugin::{get_signature, EncodingType, PluginDeclaration};
    use nu_protocol::Signature;
    let cwd = working_set.get_cwd();

    // Checking that the function is used with the correct name
    // Maybe this is not necessary but it is a sanity check
    if working_set.get_span_contents(spans[0]) != b"register" {
        return (
            garbage_pipeline(spans),
            Some(ParseError::UnknownState(
                "internal error: Wrong call name for parse plugin function".into(),
                span(spans),
            )),
        );
    }

    // Parsing the spans and checking that they match the register signature
    // Using a parsed call makes more sense than checking for how many spans are in the call
    // Also, by creating a call, it can be checked if it matches the declaration signature
    let (call, call_span) = match working_set.find_decl(b"register") {
        None => {
            return (
                garbage_pipeline(spans),
                Some(ParseError::UnknownState(
                    "internal error: Register declaration not found".into(),
                    span(spans),
                )),
            )
        }
        Some(decl_id) => {
            let (call, mut err) = parse_internal_call(
                working_set,
                spans[0],
                &spans[1..],
                decl_id,
                expand_aliases_denylist,
            );
            let decl = working_set.get_decl(decl_id);

            let call_span = span(spans);

            err = check_call(call_span, &decl.signature(), &call).or(err);
            if err.is_some() || call.has_flag("help") {
                return (
                    Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: call_span,
                        ty: Type::Any,
                        custom_completion: None,
                    }]),
                    err,
                );
            }

            (call, call_span)
        }
    };

    // Extracting the required arguments from the call and keeping them together in a tuple
    // The ? operator is not used because the error has to be kept to be printed in the shell
    // For that reason the values are kept in a result that will be passed at the end of this call
    let arguments = call
        .positional_nth(0)
        .map(|expr| {
            let name_expr = working_set.get_span_contents(expr.span);

            let (name, err) = unescape_unquote_string(name_expr, expr.span);

            if let Some(err) = err {
                Err(err)
            } else {
                let path = if let Some(p) = find_in_dirs(&name, working_set, &cwd, PLUGIN_DIRS_ENV)
                {
                    p
                } else {
                    return Err(ParseError::RegisteredFileNotFound(name, expr.span));
                };

                if path.exists() & path.is_file() {
                    Ok(path)
                } else {
                    Err(ParseError::RegisteredFileNotFound(
                        format!("{:?}", path),
                        expr.span,
                    ))
                }
            }
        })
        .expect("required positional has being checked")
        .and_then(|path| {
            call.get_flag_expr("encoding")
                .map(|expr| {
                    EncodingType::try_from_bytes(working_set.get_span_contents(expr.span))
                        .ok_or_else(|| {
                            ParseError::IncorrectValue(
                                "wrong encoding".into(),
                                expr.span,
                                "Encodings available: capnp and json".into(),
                            )
                        })
                })
                .expect("required named has being checked")
                .map(|encoding| (path, encoding))
        });

    // Signature is an optional value from the call and will be used to decide if
    // the plugin is called to get the signatures or to use the given signature
    let signature = call.positional_nth(1).map(|expr| {
        let signature = working_set.get_span_contents(expr.span);
        serde_json::from_slice::<Signature>(signature).map_err(|_| {
            ParseError::LabeledError(
                "Signature deserialization error".into(),
                "unable to deserialize signature".into(),
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
                        format!("{:?}", path),
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
                return (
                    Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: call_span,
                        ty: Type::Any,
                        custom_completion: None,
                    }]),
                    Some(err),
                );
            }
        },
    };

    let error = match signature {
        Some(signature) => arguments.and_then(|(path, encoding)| {
            signature.map(|signature| {
                let plugin_decl = PluginDeclaration::new(path, signature, encoding, shell);
                working_set.add_decl(Box::new(plugin_decl));
                working_set.mark_plugins_file_dirty();
            })
        }),
        None => arguments.and_then(|(path, encoding)| {
            get_signature(path.as_path(), &encoding, &shell)
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
                        let plugin_decl = PluginDeclaration::new(
                            path.clone(),
                            signature,
                            encoding.clone(),
                            shell.clone(),
                        );

                        working_set.add_decl(Box::new(plugin_decl));
                    }

                    working_set.mark_plugins_file_dirty();
                })
        }),
    }
    .err();

    (
        Pipeline::from_vec(vec![Expression {
            expr: Expr::Call(call),
            span: call_span,
            ty: Type::Nothing,
            custom_completion: None,
        }]),
        error,
    )
}

pub fn find_in_dirs(
    filename: &str,
    working_set: &StateWorkingSet,
    cwd: &str,
    dirs_env: &str,
) -> Option<PathBuf> {
    if let Ok(p) = canonicalize_with(filename, cwd) {
        Some(p)
    } else {
        let path = Path::new(filename);

        if path.is_relative() {
            if let Some(lib_dirs) = working_set.get_env_var(dirs_env) {
                if let Ok(dirs) = lib_dirs.as_list() {
                    for lib_dir in dirs {
                        if let Ok(dir) = lib_dir.as_path() {
                            if let Ok(dir_abs) = canonicalize_with(&dir, cwd) {
                                // make sure the dir is absolute path
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
