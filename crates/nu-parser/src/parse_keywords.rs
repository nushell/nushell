use nu_protocol::{
    ast::{Block, Call, Expr, Expression, ImportPattern, ImportPatternMember, Pipeline, Statement},
    engine::StateWorkingSet,
    span, DeclId, Span, SyntaxShape, Type,
};
use std::path::Path;

#[cfg(feature = "plugin")]
use nu_plugin::plugin::{get_signature, PluginDeclaration};

use crate::{
    lex, lite_parse,
    parser::{
        check_name, garbage, garbage_statement, parse, parse_block_expression,
        parse_import_pattern, parse_internal_call, parse_signature, parse_string,
    },
    ParseError,
};

pub fn parse_def_predecl(working_set: &mut StateWorkingSet, spans: &[Span]) -> Option<ParseError> {
    let name = working_set.get_span_contents(spans[0]);

    // handle "export def" same as "def"
    let (name, spans) = if name == b"export" && spans.len() >= 2 {
        (working_set.get_span_contents(spans[1]), &spans[1..])
    } else {
        (name, spans)
    };

    if name == b"def" && spans.len() >= 4 {
        let (name_expr, ..) = parse_string(working_set, spans[1]);
        let name = name_expr.as_string();

        working_set.enter_scope();
        // FIXME: because parse_signature will update the scope with the variables it sees
        // we end up parsing the signature twice per def. The first time is during the predecl
        // so that we can see the types that are part of the signature, which we need for parsing.
        // The second time is when we actually parse the body itworking_set.
        // We can't reuse the first time because the variables that are created during parse_signature
        // are lost when we exit the scope below.
        let (sig, ..) = parse_signature(working_set, spans[2]);
        let signature = sig.as_signature();
        working_set.exit_scope();

        if let (Some(name), Some(mut signature)) = (name, signature) {
            signature.name = name;
            let decl = signature.predeclare();

            if working_set.add_predecl(decl).is_some() {
                return Some(ParseError::DuplicateCommandDef(spans[1]));
            }
        }
    }

    None
}

pub fn parse_def(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
) -> (Statement, Option<ParseError>) {
    let mut error = None;
    let name = working_set.get_span_contents(spans[0]);

    if name == b"def" {
        let def_decl_id = working_set
            .find_decl(b"def")
            .expect("internal error: missing def command");

        let mut call = Box::new(Call {
            head: spans[0],
            decl_id: def_decl_id,
            positional: vec![],
            named: vec![],
        });

        if let Some(name_span) = spans.get(1) {
            let (name_expr, err) = parse_string(working_set, *name_span);
            error = error.or(err);

            let name = name_expr.as_string();
            call.positional.push(name_expr);

            if let Some(sig_span) = spans.get(2) {
                working_set.enter_scope();
                let (sig, err) = parse_signature(working_set, *sig_span);
                error = error.or(err);

                let signature = sig.as_signature();

                call.positional.push(sig);

                if let Some(block_span) = spans.get(3) {
                    let (block, err) = parse_block_expression(
                        working_set,
                        &SyntaxShape::Block(Some(vec![])),
                        *block_span,
                    );
                    error = error.or(err);

                    let block_id = block.as_block();

                    call.positional.push(block);

                    if let (Some(name), Some(mut signature), Some(block_id)) =
                        (&name, signature, block_id)
                    {
                        if let Some(decl_id) = working_set.find_decl(name.as_bytes()) {
                            let declaration = working_set.get_decl_mut(decl_id);

                            signature.name = name.clone();

                            *declaration = signature.into_block_command(block_id);
                        } else {
                            error = error.or_else(|| {
                                // FIXME: add a variant to ParseError that represents internal errors
                                Some(ParseError::UnknownState(
                                    "Internal error: Predeclaration failed to add declaration"
                                        .into(),
                                    spans[1],
                                ))
                            });
                        };
                    }
                } else {
                    let err_span = Span {
                        start: sig_span.end,
                        end: sig_span.end,
                    };

                    error = error
                        .or_else(|| Some(ParseError::MissingPositional("block".into(), err_span)));
                }
                working_set.exit_scope();

                if let Some(name) = name {
                    // It's OK if it returns None: The decl was already merged in previous parse
                    // pass.
                    working_set.merge_predecl(name.as_bytes());
                } else {
                    error = error.or_else(|| {
                        Some(ParseError::UnknownState(
                            "Could not get string from string expression".into(),
                            *name_span,
                        ))
                    });
                }
            } else {
                let err_span = Span {
                    start: name_span.end,
                    end: name_span.end,
                };

                error = error
                    .or_else(|| Some(ParseError::MissingPositional("parameters".into(), err_span)));
            }
        } else {
            let err_span = Span {
                start: spans[0].end,
                end: spans[0].end,
            };

            error = error.or_else(|| {
                Some(ParseError::MissingPositional(
                    "definition name".into(),
                    err_span,
                ))
            });
        }

        (
            Statement::Pipeline(Pipeline::from_vec(vec![Expression {
                expr: Expr::Call(call),
                span: span(spans),
                ty: Type::Unknown,
                custom_completion: None,
            }])),
            error,
        )
    } else {
        (
            garbage_statement(spans),
            Some(ParseError::UnknownState(
                "Expected structure: def <name> [] {}".into(),
                span(spans),
            )),
        )
    }
}

pub fn parse_alias(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
) -> (Statement, Option<ParseError>) {
    let name = working_set.get_span_contents(spans[0]);

    if name == b"alias" {
        if let Some((span, err)) = check_name(working_set, spans) {
            return (
                Statement::Pipeline(Pipeline::from_vec(vec![garbage(*span)])),
                Some(err),
            );
        }

        if let Some(decl_id) = working_set.find_decl(b"alias") {
            let (call, call_span, _) =
                parse_internal_call(working_set, spans[0], &spans[1..], decl_id);

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

            return (
                Statement::Pipeline(Pipeline::from_vec(vec![Expression {
                    expr: Expr::Call(call),
                    span: call_span,
                    ty: Type::Unknown,
                    custom_completion: None,
                }])),
                None,
            );
        }
    }

    (
        garbage_statement(spans),
        Some(ParseError::UnknownState(
            "internal error: alias statement unparseable".into(),
            span(spans),
        )),
    )
}

pub fn parse_export(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
) -> (Statement, Option<ParseError>) {
    let bytes = working_set.get_span_contents(spans[0]);

    if bytes == b"export" && spans.len() >= 3 {
        let export_name = working_set.get_span_contents(spans[1]);

        match export_name {
            b"def" => {
                let (stmt, err) = parse_def(working_set, &spans[1..]);

                let export_def_decl_id = working_set
                    .find_decl(b"export def")
                    .expect("internal error: missing 'export def' command");

                // Trying to warp the 'def' call into the 'export def' in a very clumsy way
                let stmt = if let Statement::Pipeline(ref pipe) = stmt {
                    if !pipe.expressions.is_empty() {
                        if let Expr::Call(ref call) = pipe.expressions[0].expr {
                            let mut call = call.clone();

                            call.head = span(&spans[0..=1]);
                            call.decl_id = export_def_decl_id;

                            Statement::Pipeline(Pipeline::from_vec(vec![Expression {
                                expr: Expr::Call(call),
                                span: span(spans),
                                ty: Type::Unknown,
                                custom_completion: None,
                            }]))
                        } else {
                            stmt
                        }
                    } else {
                        stmt
                    }
                } else {
                    stmt
                };

                (stmt, err)
            }
            _ => (
                garbage_statement(spans),
                Some(ParseError::Expected(
                    // TODO: Fill in more as they come
                    "def keyword".into(),
                    spans[1],
                )),
            ),
        }
    } else {
        (
            garbage_statement(spans),
            Some(ParseError::UnknownState(
                // TODO: fill in more export types as they come
                "Expected structure: export def [] {}".into(),
                span(spans),
            )),
        )
    }
}

pub fn parse_module_block(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> (Block, Option<ParseError>) {
    let mut error = None;

    working_set.enter_scope();

    let source = working_set.get_span_contents(span);

    let (output, err) = lex(source, span.start, &[], &[]);
    error = error.or(err);

    let (output, err) = lite_parse(&output);
    error = error.or(err);

    for pipeline in &output.block {
        if pipeline.commands.len() == 1 {
            parse_def_predecl(working_set, &pipeline.commands[0].parts);
        }
    }

    let mut exports: Vec<(Vec<u8>, DeclId)> = vec![];

    let block: Block = output
        .block
        .iter()
        .map(|pipeline| {
            if pipeline.commands.len() == 1 {
                // this one here is doing parse_statement() equivalent
                // let (stmt, err) = parse_statement(working_set, &pipeline.commands[0].parts);
                let name = working_set.get_span_contents(pipeline.commands[0].parts[0]);

                let (stmt, err) = match name {
                    b"def" => {
                        let (stmt, err) = parse_def(working_set, &pipeline.commands[0].parts);

                        (stmt, err)
                    }
                    b"export" => {
                        let (stmt, err) = parse_export(working_set, &pipeline.commands[0].parts);

                        if err.is_none() {
                            let decl_name =
                                // parts[2] is safe since it's checked in parse_export already
                                working_set.get_span_contents(pipeline.commands[0].parts[2]);

                            let decl_id = working_set
                                .find_decl(decl_name)
                                .expect("internal error: failed to find added declaration");

                            exports.push((decl_name.into(), decl_id));
                        }

                        (stmt, err)
                    }
                    _ => (
                        garbage_statement(&pipeline.commands[0].parts),
                        Some(ParseError::Expected(
                            "def or export keyword".into(),
                            pipeline.commands[0].parts[0],
                        )),
                    ),
                };

                if error.is_none() {
                    error = err;
                }

                stmt
            } else {
                error = Some(ParseError::Expected("not a pipeline".into(), span));
                garbage_statement(&[span])
            }
        })
        .into();

    working_set.exit_scope();

    (block.with_exports(exports), error)
}

pub fn parse_module(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
) -> (Statement, Option<ParseError>) {
    // TODO: Currently, module is closing over its parent scope (i.e., defs in the parent scope are
    // visible and usable in this module's scope). We want to disable that for files.

    let mut error = None;
    let bytes = working_set.get_span_contents(spans[0]);

    // parse_def() equivalent
    if bytes == b"module" && spans.len() >= 3 {
        let (module_name_expr, err) = parse_string(working_set, spans[1]);
        error = error.or(err);

        let module_name = module_name_expr
            .as_string()
            .expect("internal error: module name is not a string");

        // parse_block_expression() equivalent
        let block_span = spans[2];
        let block_bytes = working_set.get_span_contents(block_span);
        let mut start = block_span.start;
        let mut end = block_span.end;

        if block_bytes.starts_with(b"{") {
            start += 1;
        } else {
            return (
                garbage_statement(spans),
                Some(ParseError::Expected("block".into(), block_span)),
            );
        }

        if block_bytes.ends_with(b"}") {
            end -= 1;
        } else {
            error = error.or_else(|| {
                Some(ParseError::Unclosed(
                    "}".into(),
                    Span {
                        start: end,
                        end: end + 1,
                    },
                ))
            });
        }

        let block_span = Span { start, end };

        let (block, err) = parse_module_block(working_set, block_span);
        error = error.or(err);

        let block_id = working_set.add_module(&module_name, block);

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
            positional: vec![module_name_expr, block_expr],
            named: vec![],
        });

        (
            Statement::Pipeline(Pipeline::from_vec(vec![Expression {
                expr: Expr::Call(call),
                span: span(spans),
                ty: Type::Unknown,
                custom_completion: None,
            }])),
            error,
        )
    } else {
        (
            garbage_statement(spans),
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
) -> (Statement, Option<ParseError>) {
    let mut error = None;
    let bytes = working_set.get_span_contents(spans[0]);

    if bytes == b"use" && spans.len() >= 2 {
        let mut import_pattern_exprs: Vec<Expression> = vec![];
        for span in spans[1..].iter() {
            let (expr, err) = parse_string(working_set, *span);
            import_pattern_exprs.push(expr);
            error = error.or(err);
        }

        // TODO: Add checking for importing too long import patterns, e.g.:
        // > use spam foo non existent names here do not throw error
        let (import_pattern, err) = parse_import_pattern(working_set, &spans[1..]);
        error = error.or(err);

        let (import_pattern, exports) =
            if let Some(block_id) = working_set.find_module(&import_pattern.head) {
                (
                    import_pattern,
                    working_set.get_block(block_id).exports.clone(),
                )
            } else {
                // TODO: Do not close over when loading module from file
                // It could be a file
                if let Ok(module_filename) = String::from_utf8(import_pattern.head) {
                    let module_path = Path::new(&module_filename);
                    let module_name = if let Some(stem) = module_path.file_stem() {
                        stem.to_string_lossy().to_string()
                    } else {
                        return (
                            garbage_statement(spans),
                            Some(ParseError::ModuleNotFound(spans[1])),
                        );
                    };

                    if let Ok(contents) = std::fs::read(module_path) {
                        let span_start = working_set.next_span_start();
                        working_set.add_file(module_filename, &contents);
                        let span_end = working_set.next_span_start();

                        let (block, err) =
                            parse_module_block(working_set, Span::new(span_start, span_end));
                        error = error.or(err);

                        let block_id = working_set.add_module(&module_name, block);

                        (
                            ImportPattern {
                                head: module_name.into(),
                                members: import_pattern.members,
                            },
                            working_set.get_block(block_id).exports.clone(),
                        )
                    } else {
                        return (
                            garbage_statement(spans),
                            Some(ParseError::ModuleNotFound(spans[1])),
                        );
                    }
                } else {
                    return (
                        garbage_statement(spans),
                        Some(ParseError::NonUtf8(spans[1])),
                    );
                }
            };

        let exports = if import_pattern.members.is_empty() {
            exports
                .into_iter()
                .map(|(name, id)| {
                    let mut new_name = import_pattern.head.to_vec();
                    new_name.push(b' ');
                    new_name.extend(&name);
                    (new_name, id)
                })
                .collect()
        } else {
            match &import_pattern.members[0] {
                ImportPatternMember::Glob { .. } => exports,
                ImportPatternMember::Name { name, span } => {
                    let new_exports: Vec<(Vec<u8>, usize)> =
                        exports.into_iter().filter(|x| &x.0 == name).collect();

                    if new_exports.is_empty() {
                        error = error.or(Some(ParseError::ExportNotFound(*span)))
                    }

                    new_exports
                }
                ImportPatternMember::List { names } => {
                    let mut output = vec![];

                    for (name, span) in names {
                        let mut new_exports: Vec<(Vec<u8>, usize)> = exports
                            .iter()
                            .filter_map(|x| if &x.0 == name { Some(x.clone()) } else { None })
                            .collect();

                        if new_exports.is_empty() {
                            error = error.or(Some(ParseError::ExportNotFound(*span)))
                        } else {
                            output.append(&mut new_exports)
                        }
                    }

                    output
                }
            }
        };

        // Extend the current scope with the module's exports
        working_set.activate_overlay(exports);

        // Create the Use command call
        let use_decl_id = working_set
            .find_decl(b"use")
            .expect("internal error: missing use command");

        let call = Box::new(Call {
            head: spans[0],
            decl_id: use_decl_id,
            positional: import_pattern_exprs,
            named: vec![],
        });

        (
            Statement::Pipeline(Pipeline::from_vec(vec![Expression {
                expr: Expr::Call(call),
                span: span(spans),
                ty: Type::Unknown,
                custom_completion: None,
            }])),
            error,
        )
    } else {
        (
            garbage_statement(spans),
            Some(ParseError::UnknownState(
                "Expected structure: use <name>".into(),
                span(spans),
            )),
        )
    }
}

pub fn parse_hide(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
) -> (Statement, Option<ParseError>) {
    let mut error = None;
    let bytes = working_set.get_span_contents(spans[0]);

    if bytes == b"hide" && spans.len() >= 2 {
        let (name_expr, err) = parse_string(working_set, spans[1]);
        error = error.or(err);

        let (import_pattern, err) = parse_import_pattern(working_set, &spans[1..]);
        error = error.or(err);

        let (is_module, exported_names): (bool, Vec<Vec<u8>>) =
            if let Some(block_id) = working_set.find_module(&import_pattern.head) {
                (
                    true,
                    working_set
                        .get_block(block_id)
                        .exports
                        .iter()
                        .map(|(name, _)| name.clone())
                        .collect(),
                )
            } else if import_pattern.members.is_empty() {
                // The pattern head can be e.g. a function name, not just a module
                (false, vec![import_pattern.head.clone()])
            } else {
                return (
                    garbage_statement(spans),
                    Some(ParseError::ModuleNotFound(spans[1])),
                );
            };

        // This kind of inverts the import pattern matching found in parse_use()
        let names_to_hide = if import_pattern.members.is_empty() {
            if is_module {
                exported_names
                    .into_iter()
                    .map(|name| {
                        let mut new_name = import_pattern.head.to_vec();
                        new_name.push(b' ');
                        new_name.extend(&name);
                        new_name
                    })
                    .collect()
            } else {
                exported_names
            }
        } else {
            match &import_pattern.members[0] {
                ImportPatternMember::Glob { .. } => exported_names
                    .into_iter()
                    .map(|name| {
                        let mut new_name = import_pattern.head.to_vec();
                        new_name.push(b' ');
                        new_name.extend(&name);
                        new_name
                    })
                    .collect(),
                ImportPatternMember::Name { name, span } => {
                    let new_exports: Vec<Vec<u8>> = exported_names
                        .into_iter()
                        .filter(|n| n == name)
                        .map(|n| {
                            let mut new_name = import_pattern.head.to_vec();
                            new_name.push(b' ');
                            new_name.extend(&n);
                            new_name
                        })
                        .collect();

                    if new_exports.is_empty() {
                        error = error.or(Some(ParseError::ExportNotFound(*span)))
                    }

                    new_exports
                }
                ImportPatternMember::List { names } => {
                    let mut output = vec![];

                    for (name, span) in names {
                        let mut new_exports: Vec<Vec<u8>> = exported_names
                            .iter()
                            .filter_map(|n| if n == name { Some(n.clone()) } else { None })
                            .map(|n| {
                                let mut new_name = import_pattern.head.to_vec();
                                new_name.push(b' ');
                                new_name.extend(n);
                                new_name
                            })
                            .collect();

                        if new_exports.is_empty() {
                            error = error.or(Some(ParseError::ExportNotFound(*span)))
                        } else {
                            output.append(&mut new_exports)
                        }
                    }

                    output
                }
            }
        };

        for name in names_to_hide {
            // TODO: `use spam; use spam foo; hide foo` will hide both `foo` and `spam foo` since
            // they point to the same DeclId. Do we want to keep it that way?
            if working_set.hide_decl(&name).is_none() {
                error = error.or_else(|| Some(ParseError::UnknownCommand(spans[1])));
            }
        }

        // Create the Hide command call
        let hide_decl_id = working_set
            .find_decl(b"hide")
            .expect("internal error: missing hide command");

        let call = Box::new(Call {
            head: spans[0],
            decl_id: hide_decl_id,
            positional: vec![name_expr],
            named: vec![],
        });

        (
            Statement::Pipeline(Pipeline::from_vec(vec![Expression {
                expr: Expr::Call(call),
                span: span(spans),
                ty: Type::Unknown,
                custom_completion: None,
            }])),
            error,
        )
    } else {
        (
            garbage_statement(spans),
            Some(ParseError::UnknownState(
                "Expected structure: hide <name>".into(),
                span(spans),
            )),
        )
    }
}

pub fn parse_let(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
) -> (Statement, Option<ParseError>) {
    let name = working_set.get_span_contents(spans[0]);

    if name == b"let" {
        if let Some((span, err)) = check_name(working_set, spans) {
            return (
                Statement::Pipeline(Pipeline::from_vec(vec![garbage(*span)])),
                Some(err),
            );
        }

        if let Some(decl_id) = working_set.find_decl(b"let") {
            let (call, call_span, err) =
                parse_internal_call(working_set, spans[0], &spans[1..], decl_id);

            // Update the variable to the known type if we can.
            if err.is_none() {
                let var_id = call.positional[0]
                    .as_var()
                    .expect("internal error: expected variable");
                let rhs_type = call.positional[1].ty.clone();

                working_set.set_variable_type(var_id, rhs_type);
            }

            return (
                Statement::Pipeline(Pipeline::from_vec(vec![Expression {
                    expr: Expr::Call(call),
                    span: call_span,
                    ty: Type::Unknown,
                    custom_completion: None,
                }])),
                err,
            );
        }
    }
    (
        garbage_statement(spans),
        Some(ParseError::UnknownState(
            "internal error: let statement unparseable".into(),
            span(spans),
        )),
    )
}

pub fn parse_source(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
) -> (Statement, Option<ParseError>) {
    let name = working_set.get_span_contents(spans[0]);

    if name == b"source" {
        if let Some(decl_id) = working_set.find_decl(b"source") {
            // Is this the right call to be using here?
            // Some of the others (`parse_let`) use it, some of them (`parse_hide`) don't.
            let (call, call_span, err) =
                parse_internal_call(working_set, spans[0], &spans[1..], decl_id);

            // Command and one file name
            if spans.len() >= 2 {
                let name_expr = working_set.get_span_contents(spans[1]);
                if let Ok(filename) = String::from_utf8(name_expr.to_vec()) {
                    let source_file = Path::new(&filename);

                    let path = source_file;
                    let contents = std::fs::read(path);

                    if let Ok(contents) = contents {
                        // This will load the defs from the file into the
                        // working set, if it was a successful parse.
                        let (block, err) = parse(
                            working_set,
                            path.file_name().and_then(|x| x.to_str()),
                            &contents,
                            false,
                        );

                        if err.is_some() {
                            // Unsuccessful parse of file
                            return (
                                Statement::Pipeline(Pipeline::from_vec(vec![Expression {
                                    expr: Expr::Call(call),
                                    span: span(&spans[1..]),
                                    ty: Type::Unknown,
                                    custom_completion: None,
                                }])),
                                // Return the file parse error
                                err,
                            );
                        } else {
                            // Save the block into the working set
                            let block_id = working_set.add_block(block);

                            let mut call_with_block = call;

                            // Adding this expression to the positional creates a syntax highlighting error
                            // after writing `source example.nu`
                            call_with_block.positional.push(Expression {
                                expr: Expr::Int(block_id as i64),
                                span: spans[1],
                                ty: Type::Unknown,
                                custom_completion: None,
                            });

                            return (
                                Statement::Pipeline(Pipeline::from_vec(vec![Expression {
                                    expr: Expr::Call(call_with_block),
                                    span: call_span,
                                    ty: Type::Unknown,
                                    custom_completion: None,
                                }])),
                                None,
                            );
                        }
                    }
                } else {
                    return (
                        garbage_statement(spans),
                        Some(ParseError::NonUtf8(spans[1])),
                    );
                }
            }
            return (
                Statement::Pipeline(Pipeline::from_vec(vec![Expression {
                    expr: Expr::Call(call),
                    span: call_span,
                    ty: Type::Unknown,
                    custom_completion: None,
                }])),
                err,
            );
        }
    }
    (
        garbage_statement(spans),
        Some(ParseError::UnknownState(
            "internal error: source statement unparseable".into(),
            span(spans),
        )),
    )
}

#[cfg(feature = "plugin")]
pub fn parse_plugin(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
) -> (Statement, Option<ParseError>) {
    let name = working_set.get_span_contents(spans[0]);

    if name != b"register" {
        return (
            garbage_statement(spans),
            Some(ParseError::UnknownState(
                "internal error: Wrong call name for parse plugin function".into(),
                span(spans),
            )),
        );
    }

    if let Some(decl_id) = working_set.find_decl(b"register") {
        let (call, call_span, mut err) =
            parse_internal_call(working_set, spans[0], &spans[1..], decl_id);

        let error = {
            match spans.len() {
                1 => Some(ParseError::MissingPositional(
                    "plugin location".into(),
                    spans[0],
                )),
                2 => {
                    let name_expr = working_set.get_span_contents(spans[1]);
                    if let Ok(filename) = String::from_utf8(name_expr.to_vec()) {
                        let source_file = Path::new(&filename);

                        if source_file.exists() & source_file.is_file() {
                            // get signature from plugin
                            match get_signature(source_file) {
                                Err(err) => Some(ParseError::PluginError(format!("{}", err))),
                                Ok(signatures) => {
                                    for signature in signatures {
                                        // create plugin command declaration (need struct impl Command)
                                        // store declaration in working set
                                        let plugin_decl =
                                            PluginDeclaration::new(filename.clone(), signature);
                                        working_set.add_decl(Box::new(plugin_decl));
                                    }

                                    None
                                }
                            }
                        } else {
                            Some(ParseError::FileNotFound(filename))
                        }
                    } else {
                        Some(ParseError::NonUtf8(spans[1]))
                    }
                }
                _ => {
                    let span = spans[2..].iter().fold(spans[2], |acc, next| Span {
                        start: acc.start,
                        end: next.end,
                    });

                    Some(ParseError::ExtraPositional(span))
                }
            }
        };

        err = error.or(err);

        (
            Statement::Pipeline(Pipeline::from_vec(vec![Expression {
                expr: Expr::Call(call),
                span: call_span,
                ty: Type::Unknown,
                custom_completion: None,
            }])),
            err,
        )
    } else {
        (
            garbage_statement(spans),
            Some(ParseError::UnknownState(
                "internal error: Register declaration not found".into(),
                span(spans),
            )),
        )
    }
}
