use log::trace;
use nu_ansi_term::Style;
use nu_color_config::{get_matching_brackets_style, get_shape_color};
use nu_engine::env;
use nu_parser::{FlatShape, flatten_block, parse};
use nu_protocol::{
    Span,
    ast::{Block, Call, Expr, Expression, PipelineRedirection, RecordItem},
    engine::{EngineState, Stack, StateWorkingSet},
};
use reedline::{Highlighter, StyledText};
use std::sync::Arc;

pub struct NuHighlighter {
    pub engine_state: Arc<EngineState>,
    pub stack: Arc<Stack>,
}

impl Highlighter for NuHighlighter {
    fn highlight(&self, line: &str, cursor: usize) -> StyledText {
        let result = highlight_syntax(&self.engine_state, &self.stack, line, cursor);
        result.text
    }
}

// <<<<<<< HEAD
// =======
//         let config = self.stack.get_config(&self.engine_state);
//         let highlight_resolved_externals = config.highlight_resolved_externals;
//         let mut working_set = StateWorkingSet::new(&self.engine_state);
//         let block = parse(&mut working_set, None, line.as_bytes(), false);
//         let (shapes, global_span_offset) = {
//             let mut shapes = flatten_block(&working_set, &block);
//             // Highlighting externals has a config point because of concerns that using which to resolve
//             // externals may slow down things too much.
//             if highlight_resolved_externals {
//                 for (span, shape) in shapes.iter_mut() {
//                     if let FlatShape::External(aliased_command_span) = *shape {
//                         let str_contents = working_set.get_span_contents(aliased_command_span);
// >>>>>>> df798b657 (Fix highlighting of aliases to external commands)

/// Result of a syntax highlight operation
#[derive(Default)]
pub(crate) struct HighlightResult {
    /// The highlighted text
    pub(crate) text: StyledText,
    /// The span of any garbage that was highlighted
    pub(crate) found_garbage: Option<Span>,
}

fn collect_alias_env_spans(block: &Block, working_set: &StateWorkingSet) -> Vec<Span> {
    let mut spans = Vec::new();
    collect_alias_env_spans_from_block(block, working_set, &mut spans);
    spans
}

fn collect_alias_env_spans_from_block(
    block: &Block,
    working_set: &StateWorkingSet,
    spans: &mut Vec<Span>,
) {
    for pipeline in &block.pipelines {
        for element in &pipeline.elements {
            collect_alias_env_spans_from_expression(&element.expr, working_set, spans);
        }
    }
}

fn collect_alias_env_spans_from_expression(
    expr: &Expression,
    working_set: &StateWorkingSet,
    spans: &mut Vec<Span>,
) {
    match &expr.expr {
        Expr::Call(call) => {
            let decl = working_set.get_decl(call.decl_id);
            if decl.name() == "alias" {
                collect_alias_env_spans_from_alias_call(call, working_set, spans);
            } else {
                for argument in &call.arguments {
                    if let Some(arg_expr) = argument.expr() {
                        collect_alias_env_spans_from_expression(arg_expr, working_set, spans);
                    }
                }
            }
        }
        Expr::Keyword(keyword) => {
            collect_alias_env_spans_from_expression(&keyword.expr, working_set, spans)
        }
        Expr::Block(block_id)
        | Expr::RowCondition(block_id)
        | Expr::Subexpression(block_id)
        | Expr::Closure(block_id) => {
            let block = working_set.get_block(*block_id);
            collect_alias_env_spans_from_block(block, working_set, spans);
        }
        Expr::List(items) => {
            for item in items {
                collect_alias_env_spans_from_expression(item.expr(), working_set, spans);
            }
        }
        Expr::Table(table) => {
            for column in table.columns.iter() {
                collect_alias_env_spans_from_expression(column, working_set, spans);
            }
            for row in table.rows.iter() {
                for value in row.iter() {
                    collect_alias_env_spans_from_expression(value, working_set, spans);
                }
            }
        }
        Expr::Record(items) => {
            for item in items {
                match item {
                    RecordItem::Pair(key, value) => {
                        collect_alias_env_spans_from_expression(key, working_set, spans);
                        collect_alias_env_spans_from_expression(value, working_set, spans);
                    }
                    RecordItem::Spread(_, expr) => {
                        collect_alias_env_spans_from_expression(expr, working_set, spans)
                    }
                }
            }
        }
        Expr::BinaryOp(lhs, op, rhs) => {
            collect_alias_env_spans_from_expression(lhs, working_set, spans);
            collect_alias_env_spans_from_expression(op, working_set, spans);
            collect_alias_env_spans_from_expression(rhs, working_set, spans);
        }
        Expr::UnaryNot(expr) => collect_alias_env_spans_from_expression(expr, working_set, spans),
        Expr::Collect(_, expr) => collect_alias_env_spans_from_expression(expr, working_set, spans),
        Expr::ValueWithUnit(value) => {
            collect_alias_env_spans_from_expression(&value.expr, working_set, spans)
        }
        Expr::MatchBlock(matches) => {
            for (_, expr) in matches {
                collect_alias_env_spans_from_expression(expr, working_set, spans);
            }
        }
        Expr::StringInterpolation(parts) => {
            for part in parts {
                collect_alias_env_spans_from_expression(part, working_set, spans);
            }
        }
        Expr::GlobInterpolation(parts, _) => {
            for part in parts {
                collect_alias_env_spans_from_expression(part, working_set, spans);
            }
        }
        Expr::AttributeBlock(attribute_block) => {
            for attribute in &attribute_block.attributes {
                collect_alias_env_spans_from_expression(&attribute.expr, working_set, spans);
            }
            collect_alias_env_spans_from_expression(&attribute_block.item, working_set, spans);
        }
        Expr::FullCellPath(cell_path) => {
            collect_alias_env_spans_from_expression(&cell_path.head, working_set, spans);
        }
        _ => {}
    }
}

fn collect_alias_env_spans_from_alias_call(
    call: &Call,
    working_set: &StateWorkingSet,
    spans: &mut Vec<Span>,
) {
    if let Some(body_expr) = call.positional_nth(1) {
        if let Expr::Keyword(keyword) = &body_expr.expr {
            collect_alias_env_spans_from_alias_body(&keyword.expr, working_set, spans);
        }
    }
}

fn collect_alias_env_spans_from_alias_body(
    expr: &Expression,
    working_set: &StateWorkingSet,
    spans: &mut Vec<Span>,
) {
    match &expr.expr {
        Expr::Call(call) => {
            let decl = working_set.get_decl(call.decl_id);
            if decl.name() == "with-env" {
                if let Some(record_expr) = call.positional_nth(0) {
                    collect_record_pair_spans(record_expr, spans);
                }
                for argument in call.arguments.iter().skip(1) {
                    if let Some(arg_expr) = argument.expr() {
                        collect_alias_env_spans_from_expression(arg_expr, working_set, spans);
                    }
                }
            } else {
                for argument in &call.arguments {
                    if let Some(arg_expr) = argument.expr() {
                        collect_alias_env_spans_from_expression(arg_expr, working_set, spans);
                    }
                }
            }
        }
        Expr::Keyword(keyword) => {
            collect_alias_env_spans_from_alias_body(&keyword.expr, working_set, spans)
        }
        Expr::Block(block_id)
        | Expr::RowCondition(block_id)
        | Expr::Subexpression(block_id)
        | Expr::Closure(block_id) => {
            let block = working_set.get_block(*block_id);
            collect_alias_env_spans_from_block(block, working_set, spans);
        }
        Expr::List(items) => {
            for item in items {
                collect_alias_env_spans_from_alias_body(item.expr(), working_set, spans);
            }
        }
        Expr::Record(items) => {
            for item in items {
                match item {
                    RecordItem::Pair(key, value) => {
                        collect_alias_env_spans_from_alias_body(key, working_set, spans);
                        collect_alias_env_spans_from_alias_body(value, working_set, spans);
                    }
                    RecordItem::Spread(_, expr) => {
                        collect_alias_env_spans_from_alias_body(expr, working_set, spans)
                    }
                }
            }
        }
        _ => {
            collect_alias_env_spans_from_expression(expr, working_set, spans);
        }
    }
}

fn collect_record_pair_spans(record_expr: &Expression, spans: &mut Vec<Span>) {
    if let Expr::Record(items) = &record_expr.expr {
        for item in items {
            if let RecordItem::Pair(key, value) = item {
                spans.push(Span::new(key.span.start, value.span.end));
            }
        }
    }
}

fn span_contains(outer: &Span, inner: &Span) -> bool {
    outer.start <= inner.start && outer.end >= inner.end
}

pub(crate) fn highlight_syntax(
    engine_state: &EngineState,
    stack: &Stack,
    line: &str,
    cursor: usize,
) -> HighlightResult {
    trace!("highlighting: {line}");

    let config = stack.get_config(engine_state);
    let highlight_resolved_externals = config.highlight_resolved_externals;
    let mut working_set = StateWorkingSet::new(engine_state);
    let block = parse(&mut working_set, None, line.as_bytes(), false);
    let (shapes, global_span_offset) = {
        let mut shapes = flatten_block(&working_set, &block);
        // Highlighting externals has a config point because of concerns that using which to resolve
        // externals may slow down things too much.
        if highlight_resolved_externals {
            for (span, shape) in shapes.iter_mut() {
                if let FlatShape::External(aliased_command_span) = shape {
                    let str_contents = working_set.get_span_contents(**aliased_command_span);
                    let str_word = String::from_utf8_lossy(str_contents).to_string();
                    let paths = env::path_str(engine_state, stack, *span).ok();
                    let res = if let Ok(cwd) = engine_state.cwd(Some(stack)) {
                        which::which_in(str_word, paths.as_ref(), cwd).ok()
                    } else {
                        which::which_in_global(str_word, paths.as_ref())
                            .ok()
                            .and_then(|mut i| i.next())
                    };
                    if res.is_some() {
                        *shape = FlatShape::ExternalResolved;
                    }
                }
            }
        }
        (shapes, engine_state.next_span_start())
    };

    let mut result = HighlightResult::default();
    let mut last_seen_span = global_span_offset;

    let global_cursor_offset = cursor + global_span_offset;
    let alias_env_spans = collect_alias_env_spans(&block, &working_set);
    let matching_brackets_pos = find_matching_brackets(
        line,
        &working_set,
        &block,
        global_span_offset,
        global_cursor_offset,
    );

    for shape in &shapes {
        if shape.0.end <= last_seen_span
            || last_seen_span < global_span_offset
            || shape.0.start < global_span_offset
        {
            // We've already output something for this span
            // so just skip this one
            continue;
        }
        if shape.0.start > last_seen_span {
            let gap = line
                [(last_seen_span - global_span_offset)..(shape.0.start - global_span_offset)]
                .to_string();
            result.text.push((Style::new(), gap));
        }
        let span = shape.0;
        let next_token =
            line[(span.start - global_span_offset)..(span.end - global_span_offset)].to_string();

        let mut add_colored_token = |span: Span, shape: &FlatShape, text: String| {
            if alias_env_spans
                .iter()
                .any(|alias_span| span_contains(alias_span, &span))
            {
                result
                    .text
                    .push((get_shape_color("shape_external", &config), text));
            } else {
                result
                    .text
                    .push((get_shape_color(shape.as_str(), &config), text));
            }
        };

        match shape.1 {
            FlatShape::Garbage => {
                result.found_garbage.get_or_insert_with(|| {
                    Span::new(
                        span.start - global_span_offset,
                        span.end - global_span_offset,
                    )
                });
                add_colored_token(span, &shape.1, next_token)
            }
            FlatShape::Nothing => add_colored_token(span, &shape.1, next_token),
            FlatShape::Binary => add_colored_token(span, &shape.1, next_token),
            FlatShape::Bool => add_colored_token(span, &shape.1, next_token),
            FlatShape::Int => add_colored_token(span, &shape.1, next_token),
            FlatShape::Float => add_colored_token(span, &shape.1, next_token),
            FlatShape::Range => add_colored_token(span, &shape.1, next_token),
            FlatShape::InternalCall(_) => add_colored_token(span, &shape.1, next_token),
            FlatShape::External(_) => add_colored_token(span, &shape.1, next_token),
            FlatShape::ExternalArg => add_colored_token(span, &shape.1, next_token),
            FlatShape::ExternalResolved => add_colored_token(span, &shape.1, next_token),
            FlatShape::Keyword => add_colored_token(span, &shape.1, next_token),
            FlatShape::Literal => add_colored_token(span, &shape.1, next_token),
            FlatShape::Operator => add_colored_token(span, &shape.1, next_token),
            FlatShape::Signature => add_colored_token(span, &shape.1, next_token),
            FlatShape::String => add_colored_token(span, &shape.1, next_token),
            FlatShape::RawString => add_colored_token(span, &shape.1, next_token),
            FlatShape::StringInterpolation => add_colored_token(span, &shape.1, next_token),
            FlatShape::DateTime => add_colored_token(span, &shape.1, next_token),
            FlatShape::List
            | FlatShape::Table
            | FlatShape::Record
            | FlatShape::Block
            | FlatShape::Closure => {
                let shape = &shape.1;
                let spans = split_span_by_highlight_positions(
                    line,
                    span,
                    &matching_brackets_pos,
                    global_span_offset,
                );
                for (part, highlight) in spans {
                    let start = part.start - span.start;
                    let end = part.end - span.start;
                    let text = next_token[start..end].to_string();
                    let mut style = if alias_env_spans
                        .iter()
                        .any(|alias_span| span_contains(alias_span, &part))
                    {
                        get_shape_color("shape_external", &config)
                    } else {
                        get_shape_color(shape.as_str(), &config)
                    };
                    if highlight {
                        style = get_matching_brackets_style(style, &config);
                    }
                    result.text.push((style, text));
                }
            }

            FlatShape::Filepath => add_colored_token(span, &shape.1, next_token),
            FlatShape::Directory => add_colored_token(span, &shape.1, next_token),
            FlatShape::GlobInterpolation => add_colored_token(span, &shape.1, next_token),
            FlatShape::GlobPattern => add_colored_token(span, &shape.1, next_token),
            FlatShape::Variable(_) | FlatShape::VarDecl(_) => {
                add_colored_token(span, &shape.1, next_token)
            }
            FlatShape::Flag => add_colored_token(span, &shape.1, next_token),
            FlatShape::Pipe => add_colored_token(span, &shape.1, next_token),
            FlatShape::Redirection => add_colored_token(span, &shape.1, next_token),
            FlatShape::Custom(..) => add_colored_token(span, &shape.1, next_token),
            FlatShape::MatchPattern => add_colored_token(span, &shape.1, next_token),
        }
        last_seen_span = span.end;
    }

    let remainder = line[(last_seen_span - global_span_offset)..].to_string();
    if !remainder.is_empty() {
        result.text.push((Style::new(), remainder));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_ansi_term::{Color, Style};
    use nu_protocol::engine::Stack;

    fn test_engine_and_stack() -> (EngineState, Stack) {
        let engine_state =
            nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
        (engine_state, Stack::new())
    }

    fn highlight_buffer(line: &str) -> Vec<(Style, String)> {
        let (engine_state, stack) = test_engine_and_stack();
        let result = highlight_syntax(&engine_state, &stack, line, line.len());
        result.text.buffer
    }

    fn styles_for_range(buffer: &[(Style, String)], start: usize, len: usize) -> Vec<Style> {
        let mut styles = Vec::new();
        let mut pos = 0;
        let end = start + len;

        for (style, segment) in buffer {
            for ch in segment.chars() {
                let ch_len = ch.len_utf8();
                let ch_start = pos;
                let ch_end = ch_start + ch_len;

                if ch_end <= start {
                    pos = ch_end;
                    continue;
                }

                if ch_start >= end {
                    return styles;
                }

                if ch_start >= start && ch_start < end {
                    styles.push(*style);
                }

                pos = ch_end;
            }
        }

        styles
    }

    #[test]
    fn alias_env_prefix_tokens_are_command_colored() {
        let line = r#"alias test = E=ENV sh -c "echo $E""#;
        let buffer = highlight_buffer(line);
        let start = line.find("E=ENV").expect("prefix present");
        let styles = styles_for_range(&buffer, start, "E=ENV".len());

        assert_eq!(styles.len(), "E=ENV".len());
        assert_eq!(styles[0].foreground, Some(Color::Cyan));
        for style in &styles[2..] {
            assert_eq!(style.foreground, Some(Color::Cyan));
        }
    }

    #[test]
    fn pipeline_env_prefix_highlight_is_unchanged() {
        let line = r#"E=ENV sh -c "echo $E""#;
        let buffer = highlight_buffer(line);
        let styles = styles_for_range(&buffer, 0, "E=ENV".len());

        assert_eq!(styles.len(), "E=ENV".len());
        assert_eq!(styles[0].foreground, Some(Color::Green));
        for style in &styles[2..] {
            assert_eq!(style.foreground, Some(Color::Green));
        }
    }

    #[test]
    fn alias_multiple_assignments_not_env_highlighted() {
        let line = "alias t = A=1 B=2 cmd";
        let buffer = highlight_buffer(line);

        for pattern in ["A=1", "B=2"] {
            let start = line.find(pattern).expect("assignment present");
            let styles = styles_for_range(&buffer, start, pattern.len());

            assert_eq!(styles.len(), pattern.len());
            assert_eq!(styles[0].foreground, Some(Color::Cyan));
        }
    }

    #[test]
    fn alias_assignment_with_spaces_not_env_highlighted() {
        let line = r#"alias t = A="x y" cmd"#;
        let buffer = highlight_buffer(line);
        let start = line.find(r#"A="x y""#).expect("assignment present");
        let styles = styles_for_range(&buffer, start, r#"A="x y""#.len());

        assert_eq!(styles.len(), r#"A="x y""#.len());
        assert_eq!(styles[0].foreground, Some(Color::Cyan));
    }
}

fn split_span_by_highlight_positions(
    line: &str,
    span: Span,
    highlight_positions: &[usize],
    global_span_offset: usize,
) -> Vec<(Span, bool)> {
    let mut start = span.start;
    let mut result: Vec<(Span, bool)> = Vec::new();
    for pos in highlight_positions {
        if start <= *pos && pos < &span.end {
            if start < *pos {
                result.push((Span::new(start, *pos), false));
            }
            let span_str = &line[pos - global_span_offset..span.end - global_span_offset];
            let end = span_str
                .chars()
                .next()
                .map(|c| pos + get_char_length(c))
                .unwrap_or(pos + 1);
            result.push((Span::new(*pos, end), true));
            start = end;
        }
    }
    if start < span.end {
        result.push((Span::new(start, span.end), false));
    }
    result
}

fn find_matching_brackets(
    line: &str,
    working_set: &StateWorkingSet,
    block: &Block,
    global_span_offset: usize,
    global_cursor_offset: usize,
) -> Vec<usize> {
    const BRACKETS: &str = "{}[]()";

    // calculate first bracket position
    let global_end_offset = line.len() + global_span_offset;
    let global_bracket_pos =
        if global_cursor_offset == global_end_offset && global_end_offset > global_span_offset {
            // cursor is at the end of a non-empty string -- find block end at the previous position
            if let Some(last_char) = line.chars().last() {
                global_cursor_offset - get_char_length(last_char)
            } else {
                global_cursor_offset
            }
        } else {
            // cursor is in the middle of a string -- find block end at the current position
            global_cursor_offset
        };

    // check that position contains bracket
    let match_idx = global_bracket_pos - global_span_offset;
    if match_idx >= line.len()
        || !BRACKETS.contains(get_char_at_index(line, match_idx).unwrap_or_default())
    {
        return Vec::new();
    }

    // find matching bracket by finding matching block end
    let matching_block_end = find_matching_block_end_in_block(
        line,
        working_set,
        block,
        global_span_offset,
        global_bracket_pos,
    );
    if let Some(pos) = matching_block_end {
        let matching_idx = pos - global_span_offset;
        if BRACKETS.contains(get_char_at_index(line, matching_idx).unwrap_or_default()) {
            return if global_bracket_pos < pos {
                vec![global_bracket_pos, pos]
            } else {
                vec![pos, global_bracket_pos]
            };
        }
    }
    Vec::new()
}

fn find_matching_block_end_in_block(
    line: &str,
    working_set: &StateWorkingSet,
    block: &Block,
    global_span_offset: usize,
    global_cursor_offset: usize,
) -> Option<usize> {
    for p in &block.pipelines {
        for e in &p.elements {
            if e.expr.span.contains(global_cursor_offset)
                && let Some(pos) = find_matching_block_end_in_expr(
                    line,
                    working_set,
                    &e.expr,
                    global_span_offset,
                    global_cursor_offset,
                )
            {
                return Some(pos);
            }

            if let Some(redirection) = e.redirection.as_ref() {
                match redirection {
                    PipelineRedirection::Single { target, .. }
                    | PipelineRedirection::Separate { out: target, .. }
                    | PipelineRedirection::Separate { err: target, .. }
                        if target.span().contains(global_cursor_offset) =>
                    {
                        if let Some(pos) = target.expr().and_then(|expr| {
                            find_matching_block_end_in_expr(
                                line,
                                working_set,
                                expr,
                                global_span_offset,
                                global_cursor_offset,
                            )
                        }) {
                            return Some(pos);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    None
}

fn find_matching_block_end_in_expr(
    line: &str,
    working_set: &StateWorkingSet,
    expression: &Expression,
    global_span_offset: usize,
    global_cursor_offset: usize,
) -> Option<usize> {
    if expression.span.contains(global_cursor_offset) && expression.span.start >= global_span_offset
    {
        let expr_first = expression.span.start;
        let span_str = &line
            [expression.span.start - global_span_offset..expression.span.end - global_span_offset];
        let expr_last = span_str
            .chars()
            .last()
            .map(|c| expression.span.end - get_char_length(c))
            .unwrap_or(expression.span.start);

        return match &expression.expr {
            // TODO: Can't these be handled with an `_ => None` branch? Refactor
            Expr::Bool(_) => None,
            Expr::Int(_) => None,
            Expr::Float(_) => None,
            Expr::Binary(_) => None,
            Expr::Range(..) => None,
            Expr::Var(_) => None,
            Expr::VarDecl(_) => None,
            Expr::ExternalCall(..) => None,
            Expr::Operator(_) => None,
            Expr::UnaryNot(_) => None,
            Expr::Keyword(..) => None,
            Expr::ValueWithUnit(..) => None,
            Expr::DateTime(_) => None,
            Expr::Filepath(_, _) => None,
            Expr::Directory(_, _) => None,
            Expr::GlobPattern(_, _) => None,
            Expr::String(_) => None,
            Expr::RawString(_) => None,
            Expr::CellPath(_) => None,
            Expr::ImportPattern(_) => None,
            Expr::Overlay(_) => None,
            Expr::Signature(_) => None,
            Expr::MatchBlock(_) => None,
            Expr::Nothing => None,
            Expr::Garbage => None,

            Expr::AttributeBlock(ab) => ab
                .attributes
                .iter()
                .find_map(|attr| {
                    find_matching_block_end_in_expr(
                        line,
                        working_set,
                        &attr.expr,
                        global_span_offset,
                        global_cursor_offset,
                    )
                })
                .or_else(|| {
                    find_matching_block_end_in_expr(
                        line,
                        working_set,
                        &ab.item,
                        global_span_offset,
                        global_cursor_offset,
                    )
                }),

            Expr::Table(table) => {
                if expr_last == global_cursor_offset {
                    // cursor is at table end
                    Some(expr_first)
                } else if expr_first == global_cursor_offset {
                    // cursor is at table start
                    Some(expr_last)
                } else {
                    // cursor is inside table
                    table
                        .columns
                        .iter()
                        .chain(table.rows.iter().flat_map(AsRef::as_ref))
                        .find_map(|expr| {
                            find_matching_block_end_in_expr(
                                line,
                                working_set,
                                expr,
                                global_span_offset,
                                global_cursor_offset,
                            )
                        })
                }
            }

            Expr::Record(exprs) => {
                if expr_last == global_cursor_offset {
                    // cursor is at record end
                    Some(expr_first)
                } else if expr_first == global_cursor_offset {
                    // cursor is at record start
                    Some(expr_last)
                } else {
                    // cursor is inside record
                    exprs.iter().find_map(|expr| match expr {
                        RecordItem::Pair(k, v) => find_matching_block_end_in_expr(
                            line,
                            working_set,
                            k,
                            global_span_offset,
                            global_cursor_offset,
                        )
                        .or_else(|| {
                            find_matching_block_end_in_expr(
                                line,
                                working_set,
                                v,
                                global_span_offset,
                                global_cursor_offset,
                            )
                        }),
                        RecordItem::Spread(_, record) => find_matching_block_end_in_expr(
                            line,
                            working_set,
                            record,
                            global_span_offset,
                            global_cursor_offset,
                        ),
                    })
                }
            }

            Expr::Call(call) => call.arguments.iter().find_map(|arg| {
                arg.expr().and_then(|expr| {
                    find_matching_block_end_in_expr(
                        line,
                        working_set,
                        expr,
                        global_span_offset,
                        global_cursor_offset,
                    )
                })
            }),

            Expr::FullCellPath(b) => find_matching_block_end_in_expr(
                line,
                working_set,
                &b.head,
                global_span_offset,
                global_cursor_offset,
            ),

            Expr::BinaryOp(lhs, op, rhs) => [lhs, op, rhs].into_iter().find_map(|expr| {
                find_matching_block_end_in_expr(
                    line,
                    working_set,
                    expr,
                    global_span_offset,
                    global_cursor_offset,
                )
            }),

            Expr::Collect(_, expr) => find_matching_block_end_in_expr(
                line,
                working_set,
                expr,
                global_span_offset,
                global_cursor_offset,
            ),

            Expr::Block(block_id)
            | Expr::Closure(block_id)
            | Expr::RowCondition(block_id)
            | Expr::Subexpression(block_id) => {
                if expr_last == global_cursor_offset {
                    // cursor is at block end
                    Some(expr_first)
                } else if expr_first == global_cursor_offset {
                    // cursor is at block start
                    Some(expr_last)
                } else {
                    // cursor is inside block
                    let nested_block = working_set.get_block(*block_id);
                    find_matching_block_end_in_block(
                        line,
                        working_set,
                        nested_block,
                        global_span_offset,
                        global_cursor_offset,
                    )
                }
            }

            Expr::StringInterpolation(exprs) | Expr::GlobInterpolation(exprs, _) => {
                exprs.iter().find_map(|expr| {
                    find_matching_block_end_in_expr(
                        line,
                        working_set,
                        expr,
                        global_span_offset,
                        global_cursor_offset,
                    )
                })
            }

            Expr::List(list) => {
                if expr_last == global_cursor_offset {
                    // cursor is at list end
                    Some(expr_first)
                } else if expr_first == global_cursor_offset {
                    // cursor is at list start
                    Some(expr_last)
                } else {
                    list.iter().find_map(|item| {
                        find_matching_block_end_in_expr(
                            line,
                            working_set,
                            item.expr(),
                            global_span_offset,
                            global_cursor_offset,
                        )
                    })
                }
            }
        };
    }
    None
}

fn get_char_at_index(s: &str, index: usize) -> Option<char> {
    s[index..].chars().next()
}

fn get_char_length(c: char) -> usize {
    c.to_string().len()
}
