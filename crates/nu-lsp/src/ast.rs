use crate::Id;
use nu_protocol::{
    ast::{Argument, Block, Call, Expr, Expression, FindMapResult, ListItem, PathMember, Traverse},
    engine::StateWorkingSet,
    Span,
};
use std::sync::Arc;

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

fn try_find_id_in_misc(
    call: &Call,
    working_set: &StateWorkingSet,
    location: Option<&usize>,
    id_ref: Option<&Id>,
) -> Option<(Id, Span)> {
    let call_name = working_set.get_decl(call.decl_id).name();
    match call_name {
        "def" | "export def" => try_find_id_in_def(call, working_set, location, id_ref),
        "module" | "export module" => try_find_id_in_mod(call, working_set, location, id_ref),
        "use" | "export use" | "hide" => {
            try_find_id_in_use(call, working_set, location, id_ref, call_name)
        }
        "overlay use" | "overlay hide" => {
            try_find_id_in_overlay(call, working_set, location, id_ref)
        }
        _ => None,
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
    let mut span = None;
    for arg in call.arguments.iter() {
        if location.is_none_or(|pos| arg.span().contains(*pos)) {
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
    let decl_id = Id::Declaration(working_set.find_decl(name).or_else(|| {
        // for defs inside def
        // TODO: get scope by position
        // https://github.com/nushell/nushell/issues/15291
        (0..working_set.num_decls()).find_map(|id| {
            let decl_id = nu_protocol::DeclId::new(id);
            let decl = working_set.get_decl(decl_id);
            let span = working_set.get_block(decl.block_id()?).span?;
            call.span().contains_span(span).then_some(decl_id)
        })
    })?);
    id_ref
        .is_none_or(|id_r| decl_id == *id_r)
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
    let check_location = |span: &Span| location.is_none_or(|pos| span.contains(*pos));

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
                    .is_none_or(|id_r| found_id == *id_r)
                    .then_some((found_id, found_span))
            }
            _ => None,
        }
    })
}

/// Find id in use/hide command
/// `hide foo.nu bar` or `use foo.nu [bar baz]`
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
    call_name: &str,
) -> Option<(Id, Span)> {
    // TODO: for keyword `hide`, the decl/var is already hidden in working_set,
    // this function will always return None.
    let find_by_name = |name: &[u8]| match id {
        Some(Id::Variable(var_id_ref)) => working_set
            .find_variable(name)
            .and_then(|var_id| (var_id == *var_id_ref).then_some(Id::Variable(var_id))),
        Some(Id::Declaration(decl_id_ref)) => working_set
            .find_decl(name)
            .and_then(|decl_id| (decl_id == *decl_id_ref).then_some(Id::Declaration(decl_id))),
        Some(Id::Module(module_id_ref)) => working_set
            .find_module(name)
            .and_then(|module_id| (module_id == *module_id_ref).then_some(Id::Module(module_id))),
        None => working_set
            .find_module(name)
            .map(Id::Module)
            .or(working_set.find_decl(name).map(Id::Declaration))
            .or(working_set.find_variable(name).map(Id::Variable)),
        _ => None,
    };
    let check_location = |span: &Span| location.is_none_or(|pos| span.contains(*pos));

    // Get module id if required
    let module_name = call.arguments.first()?;
    let span = module_name.span();
    if let Some(Id::Module(_)) = id {
        // still need to check the rest, if id not matched
        if let Some(res) = get_matched_module_id(working_set, span, id) {
            return Some(res);
        }
    }
    if let Some(pos) = location {
        // first argument of `use` should always be module name
        // while it is optional in `hide`
        if span.contains(*pos) && call_name != "hide" {
            return get_matched_module_id(working_set, span, id);
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
                        find_by_name(name.as_bytes())?,
                        strip_quotes(item_expr.span, working_set),
                    ))
                })
        })
    };

    let arguments = if call_name != "hide" {
        call.arguments.get(1..)?
    } else {
        call.arguments.as_slice()
    };

    for arg in arguments {
        let Argument::Positional(expr) = arg else {
            continue;
        };
        if !check_location(&expr.span) {
            continue;
        }
        let matched = match &expr.expr {
            Expr::String(name) => {
                find_by_name(name.as_bytes()).map(|id| (id, strip_quotes(expr.span, working_set)))
            }
            Expr::List(items) => search_in_list_items(items),
            Expr::FullCellPath(fcp) => {
                let Expr::List(items) = &fcp.head.expr else {
                    return None;
                };
                search_in_list_items(items)
            }
            _ => None,
        };
        if matched.is_some() || location.is_some() {
            return matched;
        }
    }
    None
}

/// Find id in use/hide command
///
/// TODO: rename of `overlay use as new_name`, `overlay use --prefix`
///
/// # Arguments
/// - `location`: None if no `contains` check required
/// - `id`: None if no id equal check required
fn try_find_id_in_overlay(
    call: &Call,
    working_set: &StateWorkingSet,
    location: Option<&usize>,
    id: Option<&Id>,
) -> Option<(Id, Span)> {
    let check_location = |span: &Span| location.is_none_or(|pos| span.contains(*pos));
    let module_from_overlay_name = |name: &str, span: Span| {
        let found_id = Id::Module(working_set.find_overlay(name.as_bytes())?.origin);
        id.is_none_or(|id_r| found_id == *id_r)
            .then_some((found_id, strip_quotes(span, working_set)))
    };
    for arg in call.arguments.iter() {
        let Argument::Positional(expr) = arg else {
            continue;
        };
        if !check_location(&expr.span) {
            continue;
        };
        let matched = match &expr.expr {
            Expr::String(name) => get_matched_module_id(working_set, expr.span, id)
                .or(module_from_overlay_name(name, expr.span)),
            // keyword 'as'
            Expr::Keyword(kwd) => match &kwd.expr.expr {
                Expr::String(name) => module_from_overlay_name(name, kwd.expr.span),
                _ => None,
            },
            _ => None,
        };
        if matched.is_some() || location.is_some() {
            return matched;
        }
    }
    None
}

fn get_matched_module_id(
    working_set: &StateWorkingSet,
    span: Span,
    id: Option<&Id>,
) -> Option<(Id, Span)> {
    let span = strip_quotes(span, working_set);
    let name = String::from_utf8_lossy(working_set.get_span_contents(span));
    let path = std::path::PathBuf::from(name.as_ref());
    let stem = path.file_stem().and_then(|fs| fs.to_str()).unwrap_or(&name);
    let found_id = Id::Module(working_set.find_module(stem.as_bytes())?);
    id.is_none_or(|id_r| found_id == *id_r)
        .then_some((found_id, span))
}

fn find_id_in_expr(
    expr: &Expression,
    working_set: &StateWorkingSet,
    location: &usize,
) -> FindMapResult<(Id, Span)> {
    // skip the entire expression if the location is not in it
    if !expr.span.contains(*location) {
        return FindMapResult::Stop;
    }
    let span = expr.span;
    match &expr.expr {
        Expr::VarDecl(var_id) => FindMapResult::Found((Id::Variable(*var_id), span)),
        // trim leading `$` sign
        Expr::Var(var_id) => FindMapResult::Found((
            Id::Variable(*var_id),
            Span::new(span.start.saturating_add(1), span.end),
        )),
        Expr::Call(call) => {
            if call.head.contains(*location) {
                FindMapResult::Found((Id::Declaration(call.decl_id), call.head))
            } else {
                try_find_id_in_misc(call, working_set, Some(location), None)
                    .map(FindMapResult::Found)
                    .unwrap_or_default()
            }
        }
        Expr::ExternalCall(head, _) => {
            if head.span.contains(*location) {
                if let Expr::GlobPattern(cmd, _) = &head.expr {
                    return FindMapResult::Found((Id::External(cmd.clone()), head.span));
                }
            }
            FindMapResult::Continue
        }
        Expr::FullCellPath(fcp) => {
            if fcp.head.span.contains(*location) {
                FindMapResult::Continue
            } else {
                let Expression {
                    expr: Expr::Var(var_id),
                    ..
                } = fcp.head
                else {
                    return FindMapResult::Continue;
                };
                let tail: Vec<PathMember> = fcp
                    .tail
                    .clone()
                    .into_iter()
                    .take_while(|pm| pm.span().start <= *location)
                    .collect();
                let Some(span) = tail.last().map(|pm| pm.span()) else {
                    return FindMapResult::Stop;
                };
                FindMapResult::Found((Id::CellPath(var_id, tail), span))
            }
        }
        Expr::Overlay(Some(module_id)) => FindMapResult::Found((Id::Module(*module_id), span)),
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
        | Expr::String(_) => FindMapResult::Found((Id::Value(expr.ty.clone()), span)),
        _ => FindMapResult::Continue,
    }
}

/// find the leaf node at the given location from ast
pub(crate) fn find_id(
    ast: &Arc<Block>,
    working_set: &StateWorkingSet,
    location: &usize,
) -> Option<(Id, Span)> {
    let closure = |e| find_id_in_expr(e, working_set, location);
    ast.find_map(working_set, &closure)
}

fn find_reference_by_id_in_expr(
    expr: &Expression,
    working_set: &StateWorkingSet,
    id: &Id,
) -> Option<Vec<Span>> {
    let closure = |e| find_reference_by_id_in_expr(e, working_set, id);
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
                .flat_map(|e| e.flat_map(working_set, &closure))
                .collect();
            if matches!(id, Id::Declaration(decl_id) if call.decl_id == *decl_id) {
                occurs.push(call.head);
                return Some(occurs);
            }
            if let Some((_, span_found)) = try_find_id_in_misc(call, working_set, None, Some(id)) {
                occurs.push(span_found);
            }
            Some(occurs)
        }
        _ => None,
    }
}

pub(crate) fn find_reference_by_id(
    ast: &Arc<Block>,
    working_set: &StateWorkingSet,
    id: &Id,
) -> Vec<Span> {
    ast.flat_map(working_set, &|e| {
        find_reference_by_id_in_expr(e, working_set, id)
    })
}
