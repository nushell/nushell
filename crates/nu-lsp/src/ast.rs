use std::sync::Arc;

use nu_protocol::{
    ast::{
        Block, Expr, Expression, ExternalArgument, ListItem, MatchPattern, Pattern,
        PipelineRedirection, RecordItem,
    },
    engine::StateWorkingSet,
    ModuleId, VarId,
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

fn find_id_in_expr(expr: &Expression, _: &StateWorkingSet, location: &usize) -> Option<Vec<Id>> {
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
    match &expr.expr {
        Expr::Var(var_id) | Expr::VarDecl(var_id) => {
            Some(vec![Id::Variable(VarId::new(var_id.get()))])
        }
        Expr::Call(call) => {
            if call.head.contains(*location) {
                Some(vec![Id::Declaration(call.decl_id)])
            } else {
                None
            }
        }
        Expr::Overlay(Some(module_id)) => Some(vec![Id::Module(ModuleId::new(module_id.get()))]),
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
        | Expr::String(_) => Some(vec![Id::Value(expr.ty.clone())]),
        _ => None,
    }
}

/// find the leaf node at the given location from ast
pub fn find_id(ast: &Arc<Block>, working_set: &StateWorkingSet, location: &usize) -> Option<Id> {
    ast_flat_map(ast, working_set, location, find_id_in_expr)
        .first()
        .cloned()
}
