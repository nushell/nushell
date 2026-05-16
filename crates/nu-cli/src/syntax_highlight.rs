use log::trace;
use nu_ansi_term::Style;
use nu_color_config::{get_matching_brackets_style, get_shape_color};
use nu_engine::env;
use nu_parser::{FlatShape, flatten_block, parse};
use nu_protocol::{
    Span,
    ast::{Block, Expr, Expression, PipelineRedirection, RecordItem},
    engine::{EngineState, Stack, StateWorkingSet},
};
use reedline::{Highlighter, StyledText};
use std::sync::{Arc, Mutex};

struct HighlightCache {
    line: String,
    global_span_offset: usize,
    shapes: Arc<Vec<(Span, FlatShape)>>,
}

pub struct NuHighlighter {
    pub engine_state: Arc<EngineState>,
    pub stack: Arc<Stack>,
    cache: Mutex<Option<HighlightCache>>,
}

impl NuHighlighter {
    pub fn new(engine_state: Arc<EngineState>, stack: Arc<Stack>) -> Self {
        Self {
            engine_state,
            stack,
            cache: Mutex::new(None),
        }
    }
}

impl Highlighter for NuHighlighter {
    fn highlight(&self, line: &str, cursor: usize) -> StyledText {
        let result = highlight_syntax(&self.engine_state, &self.stack, line, cursor);
        *self.cache.lock().unwrap_or_else(|e| e.into_inner()) = Some(HighlightCache {
            line: line.to_string(),
            global_span_offset: result.global_span_offset,
            shapes: Arc::new(result.shapes),
        });
        result.text
    }

    fn is_inside_string_literal(&self, line: &str, cursor: usize) -> bool {
        let (global_span_offset, shapes) = match self
            .cache
            .lock()
            .ok()
            .as_deref()
            .and_then(|c| c.as_ref())
            .filter(|c| c.line == line)
        {
            Some(c) => (c.global_span_offset, Arc::clone(&c.shapes)),
            None => {
                let mut working_set = StateWorkingSet::new(&self.engine_state);
                let block = parse(&mut working_set, None, line.as_bytes(), false);
                (
                    self.engine_state.next_span_start(),
                    Arc::new(flatten_block(&working_set, &block)),
                )
            }
        };

        let global_cursor = cursor + global_span_offset;
        shapes.iter().any(|(span, shape)| {
            span.contains(global_cursor)
                && matches!(
                    shape,
                    FlatShape::String
                        | FlatShape::RawString
                        | FlatShape::StringInterpolation
                        | FlatShape::ExternalArg
                )
        })
    }
}

/// Result of a syntax highlight operation
#[derive(Default)]
pub(crate) struct HighlightResult {
    pub(crate) text: StyledText,
    pub(crate) found_garbage: Option<Span>,
    pub(crate) global_span_offset: usize,
    pub(crate) shapes: Vec<(Span, FlatShape)>,
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
    // TODO: Traverse::flat_map based highlighting?
    let shapes = flatten_block(&working_set, &block);
    let global_span_offset = engine_state.next_span_start();
    let mut result = HighlightResult {
        global_span_offset,
        ..Default::default()
    };
    let mut last_seen_span_end = global_span_offset;

    let global_cursor_offset = cursor + global_span_offset;
    let matching_brackets_pos = find_matching_brackets(
        line,
        &working_set,
        &block,
        global_span_offset,
        global_cursor_offset,
    );

    for (raw_span, flat_shape) in &shapes {
        // NOTE: Currently we expand aliases while flattening for tasks such as completion
        // https://github.com/nushell/nushell/issues/16944
        let span = if let FlatShape::External(alias_span) = flat_shape {
            alias_span
        } else {
            raw_span
        };

        if span.end <= last_seen_span_end
            || last_seen_span_end < global_span_offset
            || span.start < global_span_offset
        {
            // We've already output something for this span
            // so just skip this one
            continue;
        }
        if span.start > last_seen_span_end {
            let gap = line
                [(last_seen_span_end - global_span_offset)..(span.start - global_span_offset)]
                .to_string();
            result.text.push((Style::new(), gap));
        }
        let next_token =
            line[(span.start - global_span_offset)..(span.end - global_span_offset)].to_string();

        let mut add_colored_token = |shape: &FlatShape, text: String| {
            result
                .text
                .push((get_shape_color(shape.as_str(), &config), text));
        };

        match flat_shape {
            FlatShape::Garbage => {
                result.found_garbage.get_or_insert_with(|| {
                    Span::new(
                        span.start - global_span_offset,
                        span.end - global_span_offset,
                    )
                });
                add_colored_token(flat_shape, next_token)
            }
            FlatShape::External(_) => {
                let mut true_shape = flat_shape.clone();
                // Highlighting externals has a config point because of concerns that using which to resolve
                // externals may slow down things too much.
                if highlight_resolved_externals {
                    // use `raw_span` here for aliased external calls
                    let str_contents = working_set.get_span_contents(*raw_span);
                    let str_word = String::from_utf8_lossy(str_contents).to_string();
                    let paths = env::path_str(engine_state, stack, *raw_span).ok();
                    let res = if let Ok(cwd) = engine_state.cwd(Some(stack)) {
                        which::which_in(str_word, paths.as_ref(), cwd).ok()
                    } else {
                        which::which_in_global(str_word, paths.as_ref())
                            .ok()
                            .and_then(|mut i| i.next())
                    };
                    if res.is_some() {
                        true_shape = FlatShape::ExternalResolved;
                    }
                }
                add_colored_token(&true_shape, next_token);
            }
            FlatShape::List
            | FlatShape::Table
            | FlatShape::Record
            | FlatShape::Block
            | FlatShape::Closure => {
                let spans = split_span_by_highlight_positions(
                    line,
                    *span,
                    &matching_brackets_pos,
                    global_span_offset,
                );
                for (part, highlight) in spans {
                    let start = part.start - span.start;
                    let end = part.end - span.start;
                    let text = next_token[start..end].to_string();
                    let mut style = get_shape_color(flat_shape.as_str(), &config);
                    if highlight {
                        style = get_matching_brackets_style(style, &config);
                    }
                    result.text.push((style, text));
                }
            }
            _ => add_colored_token(flat_shape, next_token),
        }
        last_seen_span_end = span.end;
    }

    let remainder = line[(last_seen_span_end - global_span_offset)..].to_string();
    if !remainder.is_empty() {
        result.text.push((Style::new(), remainder));
    }

    result.shapes = shapes;
    result
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

#[cfg(test)]
mod tests {
    use super::NuHighlighter;
    use nu_protocol::engine::{EngineState, Stack};
    use reedline::Highlighter;
    use rstest::rstest;
    use std::sync::Arc;

    fn make_highlighter() -> NuHighlighter {
        NuHighlighter::new(Arc::new(EngineState::new()), Arc::new(Stack::new()))
    }

    #[rstest]
    // 4-byte emoji
    #[case("\"hello 🎉\" hi", 7, true)] // first byte of 🎉
    #[case("\"hello 🎉\" hi", 9, true)] // third byte of 🎉
    #[case("\"hello 🎉\" hi", 13, false)] // after closing quote
    // 8-byte zwj emoji
    #[case("\"hello 🤝🏿\" hi", 9, true)] // inside 🤝
    #[case("\"hello 🤝🏿\" hi", 11, true)] // first byte of 🏿
    #[case("\"hello 🤝🏿\" hi", 13, true)] // inside 🏿
    #[case("\"hello 🤝🏿\" hi", 17, false)] // after closing quote
    // 3-byte unicode
    #[case("\"こんにちは\" hi", 2, true)] // inside こ
    #[case("\"こんにちは\" hi", 5, true)] // inside ん
    #[case("\"こんにちは\" hi", 13, true)] // start of は
    #[case("\"こんにちは\" hi", 18, false)] // after closing quote
    // raw string
    #[case("r#'hello'# hi", 4, true)] // inside 'e'
    #[case("r#'hello'# hi", 11, false)] // after closing #
    // string interpolation
    #[case("$\"hello\" hi", 0, true)] // $ — opening StringInterpolation span (0..2)
    #[case("$\"hello\" hi", 4, true)] // inside literal 'hello'
    #[case("$\"hello\" hi", 9, false)] // after closing quote
    // no string
    #[case("1 + 2", 0, false)]
    #[case("1 + 2", 2, false)]
    // ExternalArg is treated as a string literal to suppress abbreviation expansion in external commands
    #[case("ls -la", 0, false)] // on 'ls'  — FlatShape::External
    #[case("ls -la", 3, true)] // on '-la' — FlatShape::ExternalArg
    #[case("bash -c \"echo hello\"", 0, false)] // on 'bash'            — FlatShape::External
    #[case("bash -c \"echo hello\"", 5, true)] // on '-c'              — FlatShape::ExternalArg
    #[case("bash -c \"echo hello\"", 10, true)] // inside "echo hello"  — FlatShape::ExternalArg
    fn test_is_inside_string_literal(
        #[case] line: &str,
        #[case] cursor: usize,
        #[case] expected: bool,
    ) {
        let h = make_highlighter();
        assert_eq!(h.is_inside_string_literal(line, cursor), expected);
    }
}
