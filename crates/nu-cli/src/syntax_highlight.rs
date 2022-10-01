use log::trace;
use nu_ansi_term::Style;
use nu_color_config::get_shape_color;
use nu_parser::{flatten_block, parse, FlatShape};
use nu_protocol::ast::{Argument, Block, Expr, Expression};
use nu_protocol::engine::{EngineState, StateWorkingSet};
use nu_protocol::Config;
use reedline::{Highlighter, StyledText};

pub struct NuHighlighter {
    pub engine_state: EngineState,
    pub config: Config,
}

impl Highlighter for NuHighlighter {
    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
        trace!("highlighting: {}", line);

        let mut working_set = StateWorkingSet::new(&self.engine_state);
        let block = {
            let (block, _) = parse(&mut working_set, None, line.as_bytes(), false, &[]);
            block
        };
        let (shapes, global_span_offset) = {
            let shapes = flatten_block(&working_set, &block);
            (shapes, self.engine_state.next_span_start())
        };

        let mut output = StyledText::default();
        let mut last_seen_span = global_span_offset;

        let global_cursor_offset = _cursor + global_span_offset;
        let matching_bracket_pos =
            find_matching_bracket_in_block(&working_set, &block, global_cursor_offset);

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
                output.push((Style::new(), gap));
            }
            let next_token = line
                [(shape.0.start - global_span_offset)..(shape.0.end - global_span_offset)]
                .to_string();

            macro_rules! add_colored_token_with_bracket_highlight {
                ($shape:expr, $span:expr, $text:expr) => {{
                    if let Some(pos) = matching_bracket_pos {
                        if $span.contains(pos) {
                            let pos_in_text = pos - $span.start;
                            if pos_in_text > 0 {
                                let left = (&next_token[..pos_in_text]).to_string();
                                add_colored_token!($shape, left);
                            }
                            let highlighted =
                                (&next_token[pos_in_text..pos_in_text + 1]).to_string();
                            output.push((
                                get_shape_color($shape.to_string(), &self.config).reverse(),
                                highlighted,
                            ));
                            if pos_in_text < next_token.len() {
                                let right = (&next_token[pos_in_text + 1..]).to_string();
                                add_colored_token!($shape, right);
                            }
                        } else {
                            add_colored_token!($shape, next_token);
                        }
                    } else {
                        add_colored_token!($shape, next_token);
                    }
                }};
            }

            macro_rules! add_colored_token {
                ($shape:expr, $text:expr) => {
                    output.push((get_shape_color($shape.to_string(), &self.config), $text))
                };
            }

            match shape.1 {
                FlatShape::Garbage => add_colored_token!(shape.1, next_token),
                FlatShape::Nothing => add_colored_token!(shape.1, next_token),
                FlatShape::Binary => add_colored_token!(shape.1, next_token),
                FlatShape::Bool => add_colored_token!(shape.1, next_token),
                FlatShape::Int => add_colored_token!(shape.1, next_token),
                FlatShape::Float => add_colored_token!(shape.1, next_token),
                FlatShape::Range => add_colored_token!(shape.1, next_token),
                FlatShape::InternalCall => add_colored_token!(shape.1, next_token),
                FlatShape::External => add_colored_token!(shape.1, next_token),
                FlatShape::ExternalArg => add_colored_token!(shape.1, next_token),
                FlatShape::Literal => add_colored_token!(shape.1, next_token),
                FlatShape::Operator => add_colored_token!(shape.1, next_token),
                FlatShape::Signature => add_colored_token!(shape.1, next_token),
                FlatShape::String => add_colored_token!(shape.1, next_token),
                FlatShape::StringInterpolation => add_colored_token!(shape.1, next_token),
                FlatShape::DateTime => add_colored_token!(shape.1, next_token),
                FlatShape::List => {
                    add_colored_token_with_bracket_highlight!(shape.1, shape.0, next_token)
                }
                FlatShape::Table => {
                    add_colored_token_with_bracket_highlight!(shape.1, shape.0, next_token)
                }
                FlatShape::Record => {
                    add_colored_token_with_bracket_highlight!(shape.1, shape.0, next_token)
                }

                FlatShape::Block => {
                    add_colored_token_with_bracket_highlight!(shape.1, shape.0, next_token)
                }

                FlatShape::Filepath => add_colored_token!(shape.1, next_token),
                FlatShape::Directory => add_colored_token!(shape.1, next_token),
                FlatShape::GlobPattern => add_colored_token!(shape.1, next_token),
                FlatShape::Variable => add_colored_token!(shape.1, next_token),
                FlatShape::Flag => add_colored_token!(shape.1, next_token),
                FlatShape::Custom(..) => add_colored_token!(shape.1, next_token),
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

fn find_matching_bracket_in_block(
    working_set: &StateWorkingSet,
    block: &Block,
    global_cursor_offset: usize,
) -> Option<usize> {
    for p in &block.pipelines {
        for e in &p.expressions {
            if e.span.contains(global_cursor_offset) {
                if let Some(pos) =
                    find_matching_bracket_in_expr(working_set, e, global_cursor_offset)
                {
                    return Some(pos);
                }
            }
        }
    }
    None
}

fn find_matching_bracket_in_expr(
    working_set: &StateWorkingSet,
    expression: &Expression,
    global_cursor_offset: usize,
) -> Option<usize> {
    macro_rules! find_in_expr_or_continue {
        ($inner_expr:ident) => {
            if let Some(pos) =
                find_matching_bracket_in_expr(working_set, $inner_expr, global_cursor_offset)
            {
                return Some(pos);
            }
        };
    }

    if expression.span.contains(global_cursor_offset) {
        let expr_first = expression.span.start;
        let expr_last = expression.span.end - 1;

        return match &expression.expr {
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
            Expr::Filepath(_) => None,
            Expr::Directory(_) => None,
            Expr::GlobPattern(_) => None,
            Expr::String(_) => None,
            Expr::CellPath(_) => None,
            Expr::ImportPattern(_) => None,
            Expr::Overlay(_) => None,
            Expr::Signature(_) => None,
            Expr::Nothing => None,
            Expr::Garbage => None,

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
                    };

                    if let Some(inner_expr) = opt_expr {
                        find_in_expr_or_continue!(inner_expr);
                    }
                }
                None
            }

            Expr::FullCellPath(b) => {
                find_matching_bracket_in_expr(working_set, &b.head, global_cursor_offset)
            }

            Expr::BinaryOp(lhs, op, rhs) => {
                find_in_expr_or_continue!(lhs);
                find_in_expr_or_continue!(op);
                find_in_expr_or_continue!(rhs);
                None
            }

            Expr::Block(block_id)
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
                    find_matching_bracket_in_block(working_set, nested_block, global_cursor_offset)
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
        };
    }
    None
}
