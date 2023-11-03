use log::trace;
use nu_ansi_term::Style;
use nu_color_config::{get_matching_brackets_style, get_shape_color};
use nu_parser::{flatten_block, parse, FlatShape};
use nu_protocol::ast::{Argument, Block, Expr, Expression, PipelineElement};
use nu_protocol::engine::{EngineState, StateWorkingSet};
use nu_protocol::{Config, Span};
use reedline::{Highlighter, StyledText};
use std::sync::Arc;

pub struct NuHighlighter {
    pub engine_state: Arc<EngineState>,
    pub config: Config,
}

impl Highlighter for NuHighlighter {
    fn highlight(&self, line: &str, cursor: usize) -> StyledText {
        trace!("highlighting: {}", line);

        let mut working_set = StateWorkingSet::new(&self.engine_state);
        let block = parse(&mut working_set, None, line.as_bytes(), false);

        let shapes = flatten_block(&working_set, &block);
        let global_span_offset = self.engine_state.next_span_start();

        let mut output = StyledText::default();
        let mut last_seen_span = global_span_offset;

        let global_cursor_offset = cursor + global_span_offset;
        let matching_brackets_pos = find_matching_brackets(
            line,
            &working_set,
            &block,
            global_span_offset,
            global_cursor_offset,
        );

        for shape in &shapes {
            if shape.0.end <= last_seen_span || shape.0.start < global_span_offset {
                // We've already output something for this span
                // so just skip this one
                continue;
            }
            if shape.0.start > last_seen_span {
                let gap =
                    slice(line, last_seen_span, shape.0.start, global_span_offset).to_string();
                output.push((Style::new(), gap));
            }

            let next_token =
                slice(line, shape.0.start, shape.0.end, global_span_offset).to_string();

            match shape.1 {
                FlatShape::List
                | FlatShape::Table
                | FlatShape::Record
                | FlatShape::Block
                | FlatShape::Closure => {
                    let spans = split_span_by_highlight_positions(
                        line,
                        shape.0,
                        &matching_brackets_pos,
                        global_span_offset,
                    );

                    spans.iter().for_each(|part| {
                        let start = part.0.start - shape.0.start;
                        let end = part.0.end - shape.0.start;
                        let text = (next_token[start..end]).to_string();

                        let mut style = get_shape_color(shape.1.to_string(), &self.config);
                        if part.1 {
                            style = get_matching_brackets_style(style, &self.config);
                        }
                        output.push((style, text));
                    });
                }

                // all other non-nested shapes
                _ => {
                    output.push((
                        get_shape_color(shape.1.to_string(), &self.config),
                        next_token,
                    ));
                }
            }
            last_seen_span = shape.0.end;
        }

        let remainder = line[(last_seen_span - global_span_offset)..].to_string();
        if !remainder.is_empty() {
            output.push((Style::new(), remainder));
        }

        output
    }
}

// Splits span into sub-spans.  Tuple booleans determines, wherever the span
// should be highlighted.
fn split_span_by_highlight_positions(
    line: &str,
    span: Span,
    highlight_positions: &[usize],
    global_span_offset: usize,
) -> Vec<(Span, bool)> {
    let mut start = span.start;
    let mut result: Vec<(Span, bool)> = Vec::new();

    for pos in highlight_positions {
        if start <= *pos && *pos < span.end {
            if start < *pos {
                result.push((Span::new(start, *pos), false));
            }
            let span_str = slice(line, *pos, span.end, global_span_offset);
            let end = span_str
                .chars()
                .next()
                .map(|c| pos + c.len_utf8())
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
                global_cursor_offset - last_char.len_utf8()
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
        || !BRACKETS.contains(line.chars().nth(match_idx).unwrap_or_default())
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
        if BRACKETS.contains(line.chars().nth(matching_idx).unwrap_or_default()) {
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
            match e {
                PipelineElement::Expression(_, e)
                | PipelineElement::Redirection(_, _, e)
                | PipelineElement::And(_, e)
                | PipelineElement::Or(_, e)
                | PipelineElement::SameTargetRedirection { cmd: (_, e), .. }
                | PipelineElement::SeparateRedirection { out: (_, e), .. } => {
                    if e.span.contains(global_cursor_offset) {
                        if let Some(pos) = find_matching_block_end_in_expr(
                            line,
                            working_set,
                            e,
                            global_span_offset,
                            global_cursor_offset,
                        ) {
                            return Some(pos);
                        }
                    }
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
    macro_rules! find_in_expr_or_continue {
        ($inner_expr:ident) => {
            if let Some(pos) = find_matching_block_end_in_expr(
                line,
                working_set,
                $inner_expr,
                global_span_offset,
                global_cursor_offset,
            ) {
                return Some(pos);
            }
        };
    }

    if expression.span.contains(global_cursor_offset) && expression.span.start >= global_span_offset
    {
        let expr_first = expression.span.start;
        let span_str = slice(
            line,
            expression.span.start,
            expression.span.end,
            global_span_offset,
        );
        let expr_last = span_str
            .chars()
            .last()
            .map(|c| expression.span.end - c.len_utf8())
            .unwrap_or(expression.span.start);

        return match &expression.expr {
            Expr::Table(hdr, rows) => {
                if expr_last == global_cursor_offset {
                    // cursor is at table end
                    Some(expr_first)
                } else if expr_first == global_cursor_offset {
                    // cursor is at table start
                    Some(expr_last)
                } else {
                    // cursor is inside table
                    for inner_expr in hdr {
                        find_in_expr_or_continue!(inner_expr);
                    }
                    for row in rows {
                        for inner_expr in row {
                            find_in_expr_or_continue!(inner_expr);
                        }
                    }
                    None
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
                    for (k, v) in exprs {
                        find_in_expr_or_continue!(k);
                        find_in_expr_or_continue!(v);
                    }
                    None
                }
            }

            Expr::Call(call) => {
                for arg in &call.arguments {
                    let opt_expr = match arg {
                        Argument::Named((_, _, opt_expr)) => opt_expr.as_ref(),
                        Argument::Positional(inner_expr) => Some(inner_expr),
                        Argument::Unknown(inner_expr) => Some(inner_expr),
                    };

                    if let Some(inner_expr) = opt_expr {
                        find_in_expr_or_continue!(inner_expr);
                    }
                }
                None
            }

            Expr::FullCellPath(b) => find_matching_block_end_in_expr(
                line,
                working_set,
                &b.head,
                global_span_offset,
                global_cursor_offset,
            ),

            Expr::BinaryOp(lhs, op, rhs) => {
                find_in_expr_or_continue!(lhs);
                find_in_expr_or_continue!(op);
                find_in_expr_or_continue!(rhs);
                None
            }

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

            Expr::StringInterpolation(inner_expr) => {
                for inner_expr in inner_expr {
                    find_in_expr_or_continue!(inner_expr);
                }
                None
            }

            Expr::List(inner_expr) => {
                if expr_last == global_cursor_offset {
                    // cursor is at list end
                    Some(expr_first)
                } else if expr_first == global_cursor_offset {
                    // cursor is at list start
                    Some(expr_last)
                } else {
                    // cursor is inside list
                    for inner_expr in inner_expr {
                        find_in_expr_or_continue!(inner_expr);
                    }
                    None
                }
            }

            // all other non-nested expressions
            _ => None,
        };
    }
    None
}

fn slice(line: &str, start: usize, end: usize, offset: usize) -> &str {
    &line[start - offset..end - offset]
}
