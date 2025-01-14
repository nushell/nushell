use std::sync::Arc;

use nu_protocol::{
    ast::{
        Argument, Block, Call, Expr, Expression, ExternalArgument, ListItem, MatchPattern, Pattern,
        PipelineRedirection, RecordItem,
    },
    engine::StateWorkingSet,
    DeclId, Span,
};

use crate::Id;

/// similar to flatten_block, but allows extra map function
pub fn ast_flat_map<T, E>(
    ast: &Arc<Block>,
    working_set: &StateWorkingSet,
    extra_args: &E,
    f_special: fn(&Expression, &StateWorkingSet, &E) -> Option<Vec<T>>,
) -> Vec<T> {
    ast.pipelines
        .iter()
        .flat_map(|pipeline| {
            pipeline.elements.iter().flat_map(|element| {
                expr_flat_map(&element.expr, working_set, extra_args, f_special)
                    .into_iter()
                    .chain(
                        element
                            .redirection
                            .as_ref()
                            .map(|redir| {
                                redirect_flat_map(redir, working_set, extra_args, f_special)
                            })
                            .unwrap_or_default(),
                    )
            })
        })
        .collect()
}

/// generic function that do flat_map on an expression
/// concats all recursive results on sub-expressions
///
/// # Arguments
/// * `f_special` - function that overrides the default behavior
pub fn expr_flat_map<T, E>(
    expr: &Expression,
    working_set: &StateWorkingSet,
    extra_args: &E,
    f_special: fn(&Expression, &StateWorkingSet, &E) -> Option<Vec<T>>,
) -> Vec<T> {
    // behavior overridden by f_special
    if let Some(vec) = f_special(expr, working_set, extra_args) {
        return vec;
    }
    let recur = |expr| expr_flat_map(expr, working_set, extra_args, f_special);
    match &expr.expr {
        Expr::RowCondition(block_id)
        | Expr::Subexpression(block_id)
        | Expr::Block(block_id)
        | Expr::Closure(block_id) => {
            let block = working_set.get_block(block_id.to_owned());
            ast_flat_map(block, working_set, extra_args, f_special)
        }
        Expr::Range(range) => [&range.from, &range.next, &range.to]
            .iter()
            .filter_map(|e| e.as_ref())
            .flat_map(recur)
            .collect(),
        Expr::Call(call) => call
            .arguments
            .iter()
            .filter_map(|arg| arg.expr())
            .flat_map(recur)
            .collect(),
        Expr::ExternalCall(head, args) => recur(head)
            .into_iter()
            .chain(args.iter().flat_map(|arg| match arg {
                ExternalArgument::Regular(e) | ExternalArgument::Spread(e) => recur(e),
            }))
            .collect(),
        Expr::UnaryNot(expr) | Expr::Collect(_, expr) => recur(expr),
        Expr::BinaryOp(lhs, op, rhs) => recur(lhs)
            .into_iter()
            .chain(recur(op))
            .chain(recur(rhs))
            .collect(),
        Expr::MatchBlock(matches) => matches
            .iter()
            .flat_map(|(pattern, expr)| {
                match_pattern_flat_map(pattern, working_set, extra_args, f_special)
                    .into_iter()
                    .chain(recur(expr))
            })
            .collect(),
        Expr::List(items) => items
            .iter()
            .flat_map(|item| match item {
                ListItem::Item(expr) | ListItem::Spread(_, expr) => recur(expr),
            })
            .collect(),
        Expr::Record(items) => items
            .iter()
            .flat_map(|item| match item {
                RecordItem::Spread(_, expr) => recur(expr),
                RecordItem::Pair(key, val) => [key, val].into_iter().flat_map(recur).collect(),
            })
            .collect(),
        Expr::Table(table) => table
            .columns
            .iter()
            .flat_map(recur)
            .chain(table.rows.iter().flat_map(|row| row.iter().flat_map(recur)))
            .collect(),
        Expr::ValueWithUnit(vu) => recur(&vu.expr),
        Expr::FullCellPath(fcp) => recur(&fcp.head),
        Expr::Keyword(kw) => recur(&kw.expr),
        Expr::StringInterpolation(vec) | Expr::GlobInterpolation(vec, _) => {
            vec.iter().flat_map(recur).collect()
        }

        _ => Vec::new(),
    }
}

/// flat_map on match patterns
fn match_pattern_flat_map<T, E>(
    pattern: &MatchPattern,
    working_set: &StateWorkingSet,
    extra_args: &E,
    f_special: fn(&Expression, &StateWorkingSet, &E) -> Option<Vec<T>>,
) -> Vec<T> {
    let recur = |expr| expr_flat_map(expr, working_set, extra_args, f_special);
    let recur_match = |p| match_pattern_flat_map(p, working_set, extra_args, f_special);
    match &pattern.pattern {
        Pattern::Expression(expr) => recur(expr),
        Pattern::List(patterns) | Pattern::Or(patterns) => {
            patterns.iter().flat_map(recur_match).collect()
        }
        Pattern::Record(entries) => entries.iter().flat_map(|(_, p)| recur_match(p)).collect(),
        _ => Vec::new(),
    }
    .into_iter()
    .chain(pattern.guard.as_ref().map(|g| recur(g)).unwrap_or_default())
    .collect()
}

/// flat_map on redirections
fn redirect_flat_map<T, E>(
    redir: &PipelineRedirection,
    working_set: &StateWorkingSet,
    extra_args: &E,
    f_special: fn(&Expression, &StateWorkingSet, &E) -> Option<Vec<T>>,
) -> Vec<T> {
    let recur = |expr| expr_flat_map(expr, working_set, extra_args, f_special);
    match redir {
        PipelineRedirection::Single { target, .. } => target.expr().map(recur).unwrap_or_default(),
        PipelineRedirection::Separate { out, err } => [out, err]
            .iter()
            .filter_map(|t| t.expr())
            .flat_map(recur)
            .collect(),
    }
}

/// For situations like
/// ```nushell
/// def foo [] {}
///    # |__________ location
/// ```
/// `def` is an internal call with name/signature/closure as its arguments
///
/// # Arguments
/// - `location`: None if no `contains` check required, typically when we want to compare the `decl_id` directly
fn try_find_id_in_def(
    call: &Call,
    working_set: &StateWorkingSet,
    location: Option<&usize>,
) -> Option<(Id, Span)> {
    let call_name = String::from_utf8(working_set.get_span_contents(call.head).to_vec()).ok()?;
    if !(call_name == "def" || call_name == "export def") {
        return None;
    };
    let mut span = None;
    for arg in call.arguments.iter() {
        if location
            .map(|pos| arg.span().contains(*pos))
            .unwrap_or(true)
        {
            // String means this argument is the name
            if let Argument::Positional(expr) = arg {
                if let Expr::String(_) = &expr.expr {
                    span = Some(expr.span);
                    break;
                }
            }
            // if we do care the location,
            // reaching here means this argument is not the name
            if location.is_some() {
                return None;
            }
        }
    }
    let mut span = span?;
    // adjust span if quoted
    let text = String::from_utf8(working_set.get_span_contents(span).to_vec()).ok()?;
    if text.len() > 1
        && ((text.starts_with('"') && text.ends_with('"'))
            || (text.starts_with('\'') && text.ends_with('\'')))
    {
        span = Span::new(span.start.saturating_add(1), span.end.saturating_sub(1));
    }
    let call_span = call.span();
    // find decl_ids whose span is covered by the `def` call
    let mut matched_ids: Vec<(Id, Span)> = (0..working_set.num_decls())
        .filter_map(|id| {
            let decl_id = DeclId::new(id);
            let block_id = working_set.get_decl(decl_id).block_id()?;
            let decl_span = working_set.get_block(block_id).span?;
            // find those within the `def` call
            call_span
                .contains_span(decl_span)
                .then_some((Id::Declaration(decl_id), decl_span))
        })
        .collect();
    matched_ids.sort_by_key(|(_, s)| s.start);
    matched_ids.first().cloned().map(|(id, _)| (id, span))
}

fn find_id_in_expr(
    expr: &Expression,
    working_set: &StateWorkingSet,
    location: &usize,
) -> Option<Vec<(Id, Span)>> {
    // skip the entire expression if the location is not in it
    if !expr.span.contains(*location) {
        // TODO: the span of Keyword does not include its subsidiary expression
        // resort to `expr_flat_map` if location found in its expr
        if let Expr::Keyword(kw) = &expr.expr {
            if kw.expr.span.contains(*location) {
                return None;
            }
        }
        return Some(Vec::new());
    }
    let span = expr.span;
    match &expr.expr {
        Expr::VarDecl(var_id) => Some(vec![(Id::Variable(*var_id), span)]),
        // trim leading `$` sign
        Expr::Var(var_id) => Some(vec![(
            Id::Variable(*var_id),
            Span::new(span.start.saturating_add(1), span.end),
        )]),
        Expr::Call(call) => {
            if call.head.contains(*location) {
                Some(vec![(Id::Declaration(call.decl_id), call.head)])
            } else {
                try_find_id_in_def(call, working_set, Some(location)).map(|p| vec![p])
            }
        }
        // TODO: module id of `use`
        Expr::Overlay(Some(module_id)) => Some(vec![(Id::Module(*module_id), span)]),
        // terminal value expressions
        Expr::Bool(_)
        | Expr::Binary(_)
        | Expr::DateTime(_)
        | Expr::Directory(_, _)
        | Expr::Filepath(_, _)
        | Expr::Float(_)
        | Expr::Garbage
        | Expr::GlobPattern(_, _)
        | Expr::Int(_)
        | Expr::Nothing
        | Expr::RawString(_)
        | Expr::Signature(_)
        | Expr::String(_) => Some(vec![(Id::Value(expr.ty.clone()), span)]),
        _ => None,
    }
}

/// find the leaf node at the given location from ast
pub fn find_id(
    ast: &Arc<Block>,
    working_set: &StateWorkingSet,
    location: &usize,
) -> Option<(Id, Span)> {
    ast_flat_map(ast, working_set, location, find_id_in_expr)
        .first()
        .cloned()
}

// TODO: module id support
fn find_reference_by_id_in_expr(
    expr: &Expression,
    working_set: &StateWorkingSet,
    id: &Id,
) -> Option<Vec<Span>> {
    let recur = |expr| expr_flat_map(expr, working_set, id, find_reference_by_id_in_expr);
    match (&expr.expr, id) {
        (Expr::Var(vid1), Id::Variable(vid2)) if *vid1 == *vid2 => Some(vec![Span::new(
            // we want to exclude the `$` sign for renaming
            expr.span.start.saturating_add(1),
            expr.span.end,
        )]),
        (Expr::VarDecl(vid1), Id::Variable(vid2)) if *vid1 == *vid2 => Some(vec![expr.span]),
        (Expr::Call(call), Id::Declaration(decl_id)) => {
            let mut occurs: Vec<Span> = call
                .arguments
                .iter()
                .filter_map(|arg| arg.expr())
                .flat_map(recur)
                .collect();
            if *decl_id == call.decl_id {
                occurs.push(call.head);
            }
            if let Some((id_found, span_found)) = try_find_id_in_def(call, working_set, None) {
                if id_found == *id {
                    occurs.push(span_found);
                }
            }
            Some(occurs)
        }
        _ => None,
    }
}

pub fn find_reference_by_id(ast: &Arc<Block>, working_set: &StateWorkingSet, id: &Id) -> Vec<Span> {
    ast_flat_map(ast, working_set, id, find_reference_by_id_in_expr)
}
