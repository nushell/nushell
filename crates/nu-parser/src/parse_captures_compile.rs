use crate::{
    lex::lex, parse_helpers::PERCENT_FORCED_BUILTIN_PARSER_INFO, parse_pipelines::parse_block,
};
use log::trace;
use nu_protocol::{
    BlockId, ENV_VARIABLE_ID, IN_VARIABLE_ID, ParseError, Span, Type, VarId, ast::*,
    engine::StateWorkingSet,
};
use std::{collections::HashMap, sync::Arc};

pub fn compile_block(working_set: &mut StateWorkingSet<'_>, block: &mut Block) {
    if !working_set.parse_errors.is_empty() {
        // This means there might be a bug in the parser, since calling this function while parse
        // errors are present is a logic error. However, it's not fatal and it's best to continue
        // without doing anything.
        log::error!("compile_block called with parse errors");
        return;
    }

    match nu_engine::compile(working_set, block) {
        Ok(ir_block) => {
            block.ir_block = Some(ir_block);
        }
        Err(err) => working_set.compile_errors.push(err),
    }
}

pub fn compile_block_with_id(working_set: &mut StateWorkingSet<'_>, block_id: BlockId) {
    if !working_set.parse_errors.is_empty() {
        // This means there might be a bug in the parser, since calling this function while parse
        // errors are present is a logic error. However, it's not fatal and it's best to continue
        // without doing anything.
        log::error!("compile_block_with_id called with parse errors");
        return;
    }

    match nu_engine::compile(working_set, working_set.get_block(block_id)) {
        Ok(ir_block) => {
            working_set.get_block_mut(block_id).ir_block = Some(ir_block);
        }
        Err(err) => {
            working_set.compile_errors.push(err);
        }
    };
}

pub fn discover_captures_in_closure(
    working_set: &StateWorkingSet,
    block: &Block,
    seen: &mut Vec<VarId>,
    seen_blocks: &mut HashMap<BlockId, Vec<(VarId, Span)>>,
    output: &mut Vec<(VarId, Span)>,
) -> Result<(), ParseError> {
    for flag in &block.signature.named {
        if let Some(var_id) = flag.var_id {
            seen.push(var_id);
        }
    }

    for positional in &block.signature.required_positional {
        if let Some(var_id) = positional.var_id {
            seen.push(var_id);
        }
    }
    for positional in &block.signature.optional_positional {
        if let Some(var_id) = positional.var_id {
            seen.push(var_id);
        }
    }
    if let Some(positional) = &block.signature.rest_positional
        && let Some(var_id) = positional.var_id
    {
        seen.push(var_id);
    }

    for pipeline in &block.pipelines {
        discover_captures_in_pipeline(working_set, pipeline, seen, seen_blocks, output)?;
    }

    Ok(())
}

fn discover_captures_in_pipeline(
    working_set: &StateWorkingSet,
    pipeline: &Pipeline,
    seen: &mut Vec<VarId>,
    seen_blocks: &mut HashMap<BlockId, Vec<(VarId, Span)>>,
    output: &mut Vec<(VarId, Span)>,
) -> Result<(), ParseError> {
    for element in &pipeline.elements {
        discover_captures_in_pipeline_element(working_set, element, seen, seen_blocks, output)?;
    }

    Ok(())
}

pub fn discover_captures_in_pipeline_element(
    working_set: &StateWorkingSet,
    element: &PipelineElement,
    seen: &mut Vec<VarId>,
    seen_blocks: &mut HashMap<BlockId, Vec<(VarId, Span)>>,
    output: &mut Vec<(VarId, Span)>,
) -> Result<(), ParseError> {
    discover_captures_in_expr(working_set, &element.expr, seen, seen_blocks, output)?;

    if let Some(redirection) = element.redirection.as_ref() {
        match redirection {
            PipelineRedirection::Single { target, .. } => {
                if let Some(expr) = target.expr() {
                    discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
                }
            }
            PipelineRedirection::Separate { out, err } => {
                if let Some(expr) = out.expr() {
                    discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
                }
                if let Some(expr) = err.expr() {
                    discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
                }
            }
        }
    }

    Ok(())
}

pub fn discover_captures_in_pattern(pattern: &MatchPattern, seen: &mut Vec<VarId>) {
    match &pattern.pattern {
        Pattern::Variable(var_id) => seen.push(*var_id),
        Pattern::List(items) => {
            for item in items {
                discover_captures_in_pattern(item, seen)
            }
        }
        Pattern::Record(items) => {
            for item in items {
                discover_captures_in_pattern(&item.1, seen)
            }
        }
        Pattern::Or(patterns) => {
            for pattern in patterns {
                discover_captures_in_pattern(pattern, seen)
            }
        }
        Pattern::Rest(var_id) => seen.push(*var_id),
        Pattern::Expression(_)
        | Pattern::Value(_)
        | Pattern::IgnoreValue
        | Pattern::IgnoreRest
        | Pattern::Garbage => {}
    }
}

pub fn discover_captures_in_expr(
    working_set: &StateWorkingSet,
    expr: &Expression,
    seen: &mut Vec<VarId>,
    seen_blocks: &mut HashMap<BlockId, Vec<(VarId, Span)>>,
    output: &mut Vec<(VarId, Span)>,
) -> Result<(), ParseError> {
    match &expr.expr {
        Expr::AttributeBlock(ab) => {
            discover_captures_in_expr(working_set, &ab.item, seen, seen_blocks, output)?;
        }
        Expr::BinaryOp(lhs, _, rhs) => {
            discover_captures_in_expr(working_set, lhs, seen, seen_blocks, output)?;
            discover_captures_in_expr(working_set, rhs, seen, seen_blocks, output)?;
        }
        Expr::UnaryNot(expr) => {
            discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
        }
        Expr::Closure(block_id) => {
            let block = working_set.get_block(*block_id);
            let results = {
                let mut seen = vec![];
                let mut results = vec![];

                discover_captures_in_closure(
                    working_set,
                    block,
                    &mut seen,
                    seen_blocks,
                    &mut results,
                )?;

                for (var_id, span) in results.iter() {
                    if !seen.contains(var_id)
                        && let Some(variable) = working_set.get_variable_if_possible(*var_id)
                        && variable.mutable
                    {
                        return Err(ParseError::CaptureOfMutableVar(*span));
                    }
                }

                results
            };
            seen_blocks.insert(*block_id, results.clone());
            for (var_id, span) in results.into_iter() {
                if !seen.contains(&var_id) {
                    output.push((var_id, span))
                }
            }
        }
        Expr::Block(block_id) => {
            let block = working_set.get_block(*block_id);
            // FIXME: is this correct?
            let results = {
                let mut seen = vec![];
                let mut results = vec![];
                discover_captures_in_closure(
                    working_set,
                    block,
                    &mut seen,
                    seen_blocks,
                    &mut results,
                )?;
                results
            };

            seen_blocks.insert(*block_id, results.clone());
            for (var_id, span) in results.into_iter() {
                if !seen.contains(&var_id) {
                    output.push((var_id, span))
                }
            }
        }
        Expr::Binary(_) => {}
        Expr::Bool(_) => {}
        Expr::Call(call) => {
            if let Some(head_expr) = call.parser_info.get(PERCENT_FORCED_BUILTIN_PARSER_INFO) {
                discover_captures_in_expr(working_set, head_expr, seen, seen_blocks, output)?;
            } else {
                let decl = working_set.get_decl(call.decl_id);
                if let Some(block_id) = decl.block_id() {
                    match seen_blocks.get(&block_id) {
                        Some(capture_list) => {
                            // Push captures onto the outer closure that aren't created by that outer closure
                            for capture in capture_list {
                                if !seen.contains(&capture.0) {
                                    output.push(*capture);
                                }
                            }
                        }
                        None => {
                            let block = working_set.get_block(block_id);
                            if !block.captures.is_empty() {
                                for (capture, span) in &block.captures {
                                    if !seen.contains(capture) {
                                        output.push((*capture, *span));
                                    }
                                }
                            } else {
                                let result = {
                                    let mut seen = vec![];
                                    seen_blocks.insert(block_id, vec![]);

                                    let mut result = vec![];
                                    discover_captures_in_closure(
                                        working_set,
                                        block,
                                        &mut seen,
                                        seen_blocks,
                                        &mut result,
                                    )?;

                                    result
                                };
                                // Push captures onto the outer closure that aren't created by that outer closure
                                for capture in &result {
                                    if !seen.contains(&capture.0) {
                                        output.push(*capture);
                                    }
                                }

                                seen_blocks.insert(block_id, result);
                            }
                        }
                    }
                }
            }

            for arg in &call.arguments {
                match arg {
                    Argument::Named(named) => {
                        if let Some(arg) = &named.2 {
                            discover_captures_in_expr(working_set, arg, seen, seen_blocks, output)?;
                        }
                    }
                    Argument::Positional(expr)
                    | Argument::Unknown(expr)
                    | Argument::Spread(expr) => {
                        discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
                    }
                }
            }
        }
        Expr::CellPath(_) => {}
        Expr::DateTime(_) => {}
        Expr::ExternalCall(head, args) => {
            discover_captures_in_expr(working_set, head, seen, seen_blocks, output)?;

            for ExternalArgument::Regular(expr) | ExternalArgument::Spread(expr) in args.as_ref() {
                discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
            }
        }
        Expr::Filepath(_, _) => {}
        Expr::Directory(_, _) => {}
        Expr::Float(_) => {}
        Expr::FullCellPath(cell_path) => {
            discover_captures_in_expr(working_set, &cell_path.head, seen, seen_blocks, output)?;
        }
        Expr::ImportPattern(_) => {}
        Expr::Overlay(_) => {}
        Expr::Garbage => {}
        Expr::Nothing => {}
        Expr::GlobPattern(_, _) => {}
        Expr::Int(_) => {}
        Expr::Keyword(kw) => {
            discover_captures_in_expr(working_set, &kw.expr, seen, seen_blocks, output)?;
        }
        Expr::List(list) => {
            for item in list {
                discover_captures_in_expr(working_set, item.expr(), seen, seen_blocks, output)?;
            }
        }
        Expr::Operator(_) => {}
        Expr::Range(range) => {
            if let Some(from) = &range.from {
                discover_captures_in_expr(working_set, from, seen, seen_blocks, output)?;
            }
            if let Some(next) = &range.next {
                discover_captures_in_expr(working_set, next, seen, seen_blocks, output)?;
            }
            if let Some(to) = &range.to {
                discover_captures_in_expr(working_set, to, seen, seen_blocks, output)?;
            }
        }
        Expr::Record(items) => {
            for item in items {
                match item {
                    RecordItem::Pair(field_name, field_value) => {
                        discover_captures_in_expr(
                            working_set,
                            field_name,
                            seen,
                            seen_blocks,
                            output,
                        )?;
                        discover_captures_in_expr(
                            working_set,
                            field_value,
                            seen,
                            seen_blocks,
                            output,
                        )?;
                    }
                    RecordItem::Spread(_, record) => {
                        discover_captures_in_expr(working_set, record, seen, seen_blocks, output)?;
                    }
                }
            }
        }
        Expr::Signature(sig) => {
            // Something with a declaration, similar to a var decl, will introduce more VarIds into the stack at eval
            for pos in &sig.required_positional {
                if let Some(var_id) = pos.var_id {
                    seen.push(var_id);
                }
            }
            for pos in &sig.optional_positional {
                if let Some(var_id) = pos.var_id {
                    seen.push(var_id);
                }
            }
            if let Some(rest) = &sig.rest_positional
                && let Some(var_id) = rest.var_id
            {
                seen.push(var_id);
            }
            for named in &sig.named {
                if let Some(var_id) = named.var_id {
                    seen.push(var_id);
                }
            }
        }
        Expr::String(_) => {}
        Expr::RawString(_) => {}
        Expr::StringInterpolation(exprs) | Expr::GlobInterpolation(exprs, _) => {
            for expr in exprs {
                discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
            }
        }
        Expr::MatchBlock(match_block) => {
            for match_ in match_block {
                discover_captures_in_pattern(&match_.0, seen);
                discover_captures_in_expr(working_set, &match_.1, seen, seen_blocks, output)?;
            }
        }
        Expr::Collect(var_id, expr) => {
            seen.push(*var_id);
            discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?
        }
        Expr::RowCondition(block_id) | Expr::Subexpression(block_id) => {
            let block = working_set.get_block(*block_id);

            let results = {
                let mut results = vec![];
                let mut seen = vec![];
                discover_captures_in_closure(
                    working_set,
                    block,
                    &mut seen,
                    seen_blocks,
                    &mut results,
                )?;
                results
            };

            seen_blocks.insert(*block_id, results.clone());
            for (var_id, span) in results.into_iter() {
                if !seen.contains(&var_id) {
                    output.push((var_id, span))
                }
            }
        }
        Expr::Table(table) => {
            for header in table.columns.as_ref() {
                discover_captures_in_expr(working_set, header, seen, seen_blocks, output)?;
            }
            for row in table.rows.as_ref() {
                for cell in row.as_ref() {
                    discover_captures_in_expr(working_set, cell, seen, seen_blocks, output)?;
                }
            }
        }
        Expr::ValueWithUnit(value) => {
            discover_captures_in_expr(working_set, &value.expr, seen, seen_blocks, output)?;
        }
        Expr::Var(var_id) => {
            if (*var_id > ENV_VARIABLE_ID || *var_id == IN_VARIABLE_ID) && !seen.contains(var_id) {
                output.push((*var_id, expr.span));
            }
        }
        Expr::VarDecl(var_id) => {
            seen.push(*var_id);
        }
    }
    Ok(())
}

pub(crate) fn wrap_redirection_with_collect(
    working_set: &mut StateWorkingSet,
    target: RedirectionTarget,
) -> RedirectionTarget {
    match target {
        RedirectionTarget::File { expr, append, span } => RedirectionTarget::File {
            expr: wrap_expr_with_collect(working_set, expr),
            span,
            append,
        },
        RedirectionTarget::Pipe { span } => RedirectionTarget::Pipe { span },
    }
}

pub(crate) fn wrap_element_with_collect(
    working_set: &mut StateWorkingSet,
    element: PipelineElement,
) -> PipelineElement {
    PipelineElement {
        pipe: element.pipe,
        expr: wrap_expr_with_collect(working_set, element.expr),
        redirection: element.redirection.map(|r| match r {
            PipelineRedirection::Single { source, target } => PipelineRedirection::Single {
                source,
                target: wrap_redirection_with_collect(working_set, target),
            },
            PipelineRedirection::Separate { out, err } => PipelineRedirection::Separate {
                out: wrap_redirection_with_collect(working_set, out),
                err: wrap_redirection_with_collect(working_set, err),
            },
        }),
    }
}

pub(crate) fn wrap_expr_with_collect(
    working_set: &mut StateWorkingSet,
    mut expr: Expression,
) -> Expression {
    let span = expr.span;

    // IN_VARIABLE_ID should get replaced with a unique variable, so that we don't have to
    // execute as a closure
    let var_id = working_set.add_variable(
        b"$in".into(),
        Span::new(span.start, span.start),
        Type::Any,
        false,
    );
    expr.replace_in_variable(working_set, var_id);

    // Bind the custom `$in` variable for that particular expression
    let ty = expr.ty.clone();
    Expression::new(
        working_set,
        Expr::Collect(var_id, Box::new(expr)),
        span,
        // We can expect it to have the same result type
        ty,
    )
}

pub fn parse(
    working_set: &mut StateWorkingSet,
    fname: Option<&str>,
    contents: &[u8],
    scoped: bool,
) -> Arc<Block> {
    trace!("parse");

    let file_id = {
        let fname = fname.map(nu_path::expand_to_real_path);
        let fname = fname.as_deref().map(|p| p.to_string_lossy());
        let name = fname.as_deref().unwrap_or("source");
        working_set.add_file(name, contents)
    };

    let new_span = working_set.get_span_for_file(file_id);

    let previously_parsed_block = working_set.find_block_by_span(new_span);

    let mut output = {
        if let Some(block) = previously_parsed_block {
            return block;
        } else {
            let (output, err) = lex(contents, new_span.start, &[], &[], false);
            if let Some(err) = err {
                working_set.error(err)
            }

            Arc::new(parse_block(working_set, &output, new_span, scoped, false))
        }
    };

    // Top level `Block`s are compiled eagerly, as they don't have a parent which would cause them
    // to be compiled later.
    if working_set.parse_errors.is_empty() {
        compile_block(working_set, Arc::make_mut(&mut output));
    }

    let mut seen = vec![];
    let mut seen_blocks = HashMap::new();

    let mut captures = vec![];
    match discover_captures_in_closure(
        working_set,
        &output,
        &mut seen,
        &mut seen_blocks,
        &mut captures,
    ) {
        Ok(_) => {
            Arc::make_mut(&mut output).captures = captures;
        }
        Err(err) => working_set.error(err),
    }

    // Also check other blocks that might have been imported
    let mut errors = vec![];
    for (block_idx, block) in working_set.delta.blocks.iter().enumerate() {
        let block_id = block_idx + working_set.permanent_state.num_blocks();
        let block_id = BlockId::new(block_id);

        if !seen_blocks.contains_key(&block_id) {
            let mut captures = vec![];

            match discover_captures_in_closure(
                working_set,
                block,
                &mut seen,
                &mut seen_blocks,
                &mut captures,
            ) {
                Ok(_) => {
                    seen_blocks.insert(block_id, captures);
                }
                Err(err) => {
                    errors.push(err);
                }
            }
        }
    }
    for err in errors {
        working_set.error(err)
    }

    for (block_id, captures) in seen_blocks.into_iter() {
        // In theory, we should only be updating captures where we have new information
        // the only place where this is possible would be blocks that are newly created
        // by our working set delta. If we ever tried to modify the permanent state, we'd
        // panic (again, in theory, this shouldn't be possible)
        let block = working_set.get_block(block_id);
        let block_captures_empty = block.captures.is_empty();
        // need to check block_id >= working_set.permanent_state.num_blocks()
        // to avoid mutate a block that is in the permanent state.
        // this can happened if user defines a function with recursive call
        // and pipe a variable to the command, e.g:
        // def px [] { if true { 42 } else { px } };    # the block px is saved in permanent state.
        // let x = 3
        // $x | px
        // If we don't guard for `block_id`, it will change captures of `px`, which is
        // already saved in permanent state
        if !captures.is_empty()
            && block_captures_empty
            && block_id.get() >= working_set.permanent_state.num_blocks()
        {
            let block = working_set.get_block_mut(block_id);
            block.captures = captures;
        }
    }

    output
}
