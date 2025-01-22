use nu_protocol::{
    ast::{
        Argument, Block, Call, Expr, Expression, ExternalArgument, ListItem, MatchPattern, Pattern,
        PipelineRedirection, RecordItem,
    },
    engine::StateWorkingSet,
    Span,
};
use std::{path::PathBuf, sync::Arc};

use crate::Id;

/// similar to flatten_block, but allows extra map function
pub fn ast_flat_map<'a, T, F>(
    ast: &'a Arc<Block>,
    working_set: &'a StateWorkingSet,
    f_special: &F,
) -> Vec<T>
where
    F: Fn(&'a Expression) -> Option<Vec<T>>,
{
    ast.pipelines
        .iter()
        .flat_map(|pipeline| {
            pipeline.elements.iter().flat_map(|element| {
                expr_flat_map(&element.expr, working_set, f_special)
                    .into_iter()
                    .chain(
                        element
                            .redirection
                            .as_ref()
                            .map(|redir| redirect_flat_map(redir, working_set, f_special))
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
pub fn expr_flat_map<'a, T, F>(
    expr: &'a Expression,
    working_set: &'a StateWorkingSet,
    f_special: &F,
) -> Vec<T>
where
    F: Fn(&'a Expression) -> Option<Vec<T>>,
{
    // behavior overridden by f_special
    if let Some(vec) = f_special(expr) {
        return vec;
    }
    let recur = |expr| expr_flat_map(expr, working_set, f_special);
    match &expr.expr {
        Expr::RowCondition(block_id)
        | Expr::Subexpression(block_id)
        | Expr::Block(block_id)
        | Expr::Closure(block_id) => {
            let block = working_set.get_block(block_id.to_owned());
            ast_flat_map(block, working_set, f_special)
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
                match_pattern_flat_map(pattern, working_set, f_special)
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
fn match_pattern_flat_map<'a, T, F>(
    pattern: &'a MatchPattern,
    working_set: &'a StateWorkingSet,
    f_special: &F,
) -> Vec<T>
where
    F: Fn(&'a Expression) -> Option<Vec<T>>,
{
    let recur = |expr| expr_flat_map(expr, working_set, f_special);
    let recur_match = |p| match_pattern_flat_map(p, working_set, f_special);
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
fn redirect_flat_map<'a, T, F>(
    redir: &'a PipelineRedirection,
    working_set: &'a StateWorkingSet,
    f_special: &F,
) -> Vec<T>
where
    F: Fn(&'a Expression) -> Option<Vec<T>>,
{
    let recur = |expr| expr_flat_map(expr, working_set, f_special);
    match redir {
        PipelineRedirection::Single { target, .. } => target.expr().map(recur).unwrap_or_default(),
        PipelineRedirection::Separate { out, err } => [out, err]
            .iter()
            .filter_map(|t| t.expr())
            .flat_map(recur)
            .collect(),
    }
}

/// Adjust span if quoted
fn strip_quotes(span: Span, working_set: &StateWorkingSet) -> Span {
    let text = String::from_utf8_lossy(working_set.get_span_contents(span));
    if text.len() > 1
        && ((text.starts_with('"') && text.ends_with('"'))
            || (text.starts_with('\'') && text.ends_with('\'')))
    {
        Span::new(span.start.saturating_add(1), span.end.saturating_sub(1))
    } else {
        span
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
/// - `location`: None if no `contains` check required
/// - `id`: None if no id equal check required
fn try_find_id_in_def(
    call: &Call,
    working_set: &StateWorkingSet,
    location: Option<&usize>,
    id_ref: Option<&Id>,
) -> Option<(Id, Span)> {
    let call_name = working_set.get_span_contents(call.head);
    if call_name != b"def" && call_name != b"export def" {
        return None;
    };
    let mut span = None;
    for arg in call.arguments.iter() {
        if location.map_or(true, |pos| arg.span().contains(*pos)) {
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
    let span = strip_quotes(span?, working_set);
    let name = working_set.get_span_contents(span);
    let decl_id = Id::Declaration(working_set.find_decl(name)?);
    id_ref
        .map_or(true, |id_r| decl_id == *id_r)
        .then_some((decl_id, span))
}

/// For situations like
/// ```nushell
/// module foo {}
///       # |__________ location
/// ```
/// `module` is an internal call with name/signature/closure as its arguments
///
/// # Arguments
/// - `location`: None if no `contains` check required
/// - `id`: None if no id equal check required
fn try_find_id_in_mod(
    call: &Call,
    working_set: &StateWorkingSet,
    location: Option<&usize>,
    id_ref: Option<&Id>,
) -> Option<(Id, Span)> {
    let call_name = working_set.get_span_contents(call.head);
    if call_name != "module".as_bytes() && call_name != "export module".as_bytes() {
        return None;
    };
    let check_location = |span: &Span| location.map_or(true, |pos| span.contains(*pos));

    call.arguments.first().and_then(|arg| {
        if !check_location(&arg.span()) {
            return None;
        }
        match arg {
            Argument::Positional(expr) => {
                let name = expr.as_string()?;
                let module_id = working_set.find_module(name.as_bytes())?;
                let found_id = Id::Module(module_id);
                let found_span = strip_quotes(arg.span(), working_set);
                id_ref
                    .map_or(true, |id_r| found_id == *id_r)
                    .then_some((found_id, found_span))
            }
            _ => None,
        }
    })
}

/// Find id in use command
/// `use foo.nu bar` or `use foo.nu [bar baz]`
/// NOTE: `call.parser_info` contains a 'import_pattern' field for `use` commands,
/// but sometimes it is missing, so fall back to `call_name == "use"` here.
/// One drawback is that the `module_id` is harder to get
///
/// # Arguments
/// - `location`: None if no `contains` check required
/// - `id`: None if no id equal check required
fn try_find_id_in_use(
    call: &Call,
    working_set: &StateWorkingSet,
    location: Option<&usize>,
    id: Option<&Id>,
) -> Option<(Id, Span)> {
    let call_name = working_set.get_span_contents(call.head);
    if call_name != "use".as_bytes() {
        return None;
    }
    let find_by_name = |name: &str| {
        match id {
            Some(Id::Variable(var_id_ref)) => {
                if let Some(var_id) = working_set.find_variable(name.as_bytes()) {
                    if var_id == *var_id_ref {
                        return Some(Id::Variable(var_id));
                    }
                }
            }
            Some(Id::Declaration(decl_id_ref)) => {
                if let Some(decl_id) = working_set.find_decl(name.as_bytes()) {
                    if decl_id == *decl_id_ref {
                        return Some(Id::Declaration(decl_id));
                    }
                }
            }
            Some(Id::Module(module_id_ref)) => {
                if let Some(module_id) = working_set.find_module(name.as_bytes()) {
                    if module_id == *module_id_ref {
                        return Some(Id::Module(module_id));
                    }
                }
            }
            None => {
                if let Some(var_id) = working_set.find_variable(name.as_bytes()) {
                    return Some(Id::Variable(var_id));
                }
                if let Some(decl_id) = working_set.find_decl(name.as_bytes()) {
                    return Some(Id::Declaration(decl_id));
                }
                if let Some(module_id) = working_set.find_module(name.as_bytes()) {
                    return Some(Id::Module(module_id));
                }
            }
            _ => (),
        }
        None
    };
    let check_location = |span: &Span| location.map_or(true, |pos| span.contains(*pos));
    let get_module_id = |span: Span| {
        let span = strip_quotes(span, working_set);
        let name = String::from_utf8_lossy(working_set.get_span_contents(span));
        let path = PathBuf::from(name.as_ref());
        let stem = path.file_stem().and_then(|fs| fs.to_str()).unwrap_or(&name);
        let module_id = working_set.find_module(stem.as_bytes())?;
        let found_id = Id::Module(module_id);
        id.map_or(true, |id_r| found_id == *id_r)
            .then_some((found_id, span))
    };

    // Get module id if required
    let module_name = call.arguments.first()?;
    let span = module_name.span();
    if let Some(Id::Module(_)) = id {
        // still need to check the second argument
        if let Some(res) = get_module_id(span) {
            return Some(res);
        }
    }
    if let Some(pos) = location {
        if span.contains(*pos) {
            return get_module_id(span);
        }
    }

    let search_in_list_items = |items: &Vec<ListItem>| {
        items.iter().find_map(|item| {
            let item_expr = item.expr();
            check_location(&item_expr.span)
                .then_some(item_expr)
                .and_then(|e| {
                    let name = e.as_string()?;
                    Some((
                        find_by_name(&name)?,
                        strip_quotes(item_expr.span, working_set),
                    ))
                })
        })
    };

    // the imported name is always at the second argument
    if let Argument::Positional(expr) = call.arguments.get(1)? {
        if check_location(&expr.span) {
            match &expr.expr {
                Expr::String(name) => {
                    if let Some(id) = find_by_name(name) {
                        return Some((id, strip_quotes(expr.span, working_set)));
                    }
                }
                Expr::List(items) => {
                    if let Some(res) = search_in_list_items(items) {
                        return Some(res);
                    }
                }
                Expr::FullCellPath(fcp) => {
                    if let Expr::List(items) = &fcp.head.expr {
                        if let Some(res) = search_in_list_items(items) {
                            return Some(res);
                        }
                    }
                }
                _ => (),
            }
        }
    }
    None
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
                try_find_id_in_def(call, working_set, Some(location), None)
                    .or(try_find_id_in_mod(call, working_set, Some(location), None))
                    .or(try_find_id_in_use(call, working_set, Some(location), None))
                    .map(|p| vec![p])
            }
        }
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
    let closure = |e| find_id_in_expr(e, working_set, location);
    ast_flat_map(ast, working_set, &closure).first().cloned()
}

fn find_reference_by_id_in_expr(
    expr: &Expression,
    working_set: &StateWorkingSet,
    id: &Id,
) -> Option<Vec<Span>> {
    let closure = |e| find_reference_by_id_in_expr(e, working_set, id);
    let recur = |expr| expr_flat_map(expr, working_set, &closure);
    match (&expr.expr, id) {
        (Expr::Var(vid1), Id::Variable(vid2)) if *vid1 == *vid2 => Some(vec![Span::new(
            // we want to exclude the `$` sign for renaming
            expr.span.start.saturating_add(1),
            expr.span.end,
        )]),
        (Expr::VarDecl(vid1), Id::Variable(vid2)) if *vid1 == *vid2 => Some(vec![expr.span]),
        // also interested in `var_id` in call.arguments of `use` command
        // and `module_id` in `module` command
        (Expr::Call(call), _) => {
            let mut occurs: Vec<Span> = call
                .arguments
                .iter()
                .filter_map(|arg| arg.expr())
                .flat_map(recur)
                .collect();
            if let Id::Declaration(decl_id) = id {
                if *decl_id == call.decl_id {
                    occurs.push(call.head);
                }
            }
            if let Some((_, span_found)) = try_find_id_in_def(call, working_set, None, Some(id))
                .or(try_find_id_in_mod(call, working_set, None, Some(id)))
                .or(try_find_id_in_use(call, working_set, None, Some(id)))
            {
                occurs.push(span_found);
            }
            Some(occurs)
        }
        _ => None,
    }
}

pub fn find_reference_by_id(ast: &Arc<Block>, working_set: &StateWorkingSet, id: &Id) -> Vec<Span> {
    ast_flat_map(ast, working_set, &|e| {
        find_reference_by_id_in_expr(e, working_set, id)
    })
}
