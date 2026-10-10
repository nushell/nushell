use log::trace;
use nu_ansi_term::Style;
use nu_color_config::{get_matching_brackets_style, get_shape_color};
use nu_engine::env;
use nu_parser::{FlatShape, flatten_block, parse};
use nu_protocol::{
    BlockId, Span,
    ast::{Block, Expr, Expression, PipelineRedirection, RecordItem},
    engine::{EngineState, Stack, StateWorkingSet},
};
use reedline::{AbbrExpandContext, Highlighter, StyledText};
use std::{
    borrow::Cow,
    ffi::OsStr,
    sync::{Arc, Mutex},
};

/// A highlighter that does nothing
///
/// Used to remove highlighting from a reedline instance
/// (letting NuHighlighter structs be dropped)
#[derive(Default)]
pub struct NoOpHighlighter {}

impl Highlighter for NoOpHighlighter {
    fn highlight(&self, _line: &str, _cursor: usize) -> reedline::StyledText {
        StyledText::new()
    }
}

/// Engine bits that change span / block IDs. Overlay or file loads must miss the cache.
#[derive(Clone, Copy, PartialEq, Eq)]
struct HighlightEngineKey {
    span_start: usize,
    num_blocks: usize,
    num_decls: usize,
}

impl HighlightEngineKey {
    fn of(engine_state: &EngineState) -> Self {
        Self {
            span_start: engine_state.next_span_start(),
            num_blocks: engine_state.num_blocks(),
            num_decls: engine_state.num_decls(),
        }
    }
}

/// Parse/flatten result reused across paints of the same line.
#[derive(Clone)]
struct ParsedHighlight {
    block: Arc<Block>,
    /// Blocks added by this parse (`StateWorkingSet::delta.blocks`), in id order.
    delta_blocks: Vec<Arc<Block>>,
    shapes: Arc<Vec<(Span, FlatShape)>>,
    global_span_offset: usize,
}

struct HighlightCache {
    line: String,
    engine: HighlightEngineKey,
    parsed: ParsedHighlight,
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
        let parsed = self.parsed_line(line);
        style_parsed(&self.engine_state, &self.stack, line, cursor, &parsed).text
    }

    fn should_expand_abbr(&self, line: &str, cursor: usize, context: AbbrExpandContext) -> bool {
        let parsed = self.parsed_line(line);
        let global_cursor = cursor + parsed.global_span_offset;
        !parsed.shapes.iter().any(|(span, shape)| {
            span.contains(global_cursor)
                && match context {
                    AbbrExpandContext::WordAbbreviation => matches!(
                        shape,
                        FlatShape::String
                            | FlatShape::RawString
                            | FlatShape::StringInterpolation
                            | FlatShape::ExternalArg
                    ),
                    AbbrExpandContext::BangExpansion => false,
                }
        })
    }
}

impl NuHighlighter {
    fn parsed_line(&self, line: &str) -> ParsedHighlight {
        let engine = HighlightEngineKey::of(&self.engine_state);
        {
            let guard = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(cache) = guard.as_ref()
                && cache.line == line
                && cache.engine == engine
            {
                return cache.parsed.clone();
            }
        }

        let parsed = parse_highlight_line(&self.engine_state, line);
        *self.cache.lock().unwrap_or_else(|e| e.into_inner()) = Some(HighlightCache {
            line: line.to_string(),
            engine,
            parsed: parsed.clone(),
        });
        parsed
    }
}

/// Result of a syntax highlight operation
#[derive(Default)]
pub(crate) struct HighlightResult {
    pub(crate) text: StyledText,
    pub(crate) found_garbage: Option<Span>,
}

pub(crate) fn highlight_syntax(
    engine_state: &EngineState,
    stack: &Stack,
    line: &str,
    cursor: usize,
) -> HighlightResult {
    let parsed = parse_highlight_line(engine_state, line);
    style_parsed(engine_state, stack, line, cursor, &parsed)
}

fn parse_highlight_line(engine_state: &EngineState, line: &str) -> ParsedHighlight {
    trace!("highlighting: {line}");

    let mut working_set = StateWorkingSet::new(engine_state);
    // Color `use` without parse-time-loading modules (the `d` hitch on `use std`).
    working_set.skip_module_load = true;
    let block = parse(&mut working_set, None, line.as_bytes(), false);
    // TODO: Traverse::flat_map based highlighting?
    let shapes = flatten_block(&working_set, &block);
    ParsedHighlight {
        block,
        delta_blocks: working_set.delta.blocks,
        shapes: Arc::new(shapes),
        global_span_offset: engine_state.next_span_start(),
    }
}

fn style_parsed(
    engine_state: &EngineState,
    stack: &Stack,
    line: &str,
    cursor: usize,
    parsed: &ParsedHighlight,
) -> HighlightResult {
    let config = stack.get_config(engine_state);
    let highlight_resolved_externals = config.highlight_resolved_externals;
    let global_span_offset = parsed.global_span_offset;
    let shapes = parsed.shapes.as_ref();
    let mut result = HighlightResult::default();
    result.text.buffer.reserve(shapes.len().saturating_mul(2));
    let mut last_seen_span_end = global_span_offset;

    let get_block = |id: BlockId| get_highlight_block(engine_state, &parsed.delta_blocks, id);
    let global_cursor_offset = cursor + global_span_offset;
    let matching_brackets_pos = find_matching_brackets(
        line,
        &get_block,
        &parsed.block,
        global_span_offset,
        global_cursor_offset,
    );

    for (raw_span, flat_shape) in shapes {
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
            push_line_slice(
                &mut result.text,
                Style::new(),
                line,
                last_seen_span_end - global_span_offset,
                span.start - global_span_offset,
            );
        }

        let token_start = span.start - global_span_offset;
        let token_end = span.end - global_span_offset;

        match flat_shape {
            FlatShape::Garbage => {
                result.found_garbage.get_or_insert_with(|| {
                    Span::new(
                        span.start - global_span_offset,
                        span.end - global_span_offset,
                    )
                });
                push_shaped_slice(
                    &mut result.text,
                    flat_shape,
                    &config,
                    line,
                    token_start,
                    token_end,
                );
            }
            FlatShape::External(_) => {
                // Highlighting externals has a config point because of concerns that using which to resolve
                // externals may slow down things too much.
                let resolved = highlight_resolved_externals
                    && external_is_resolved(
                        engine_state,
                        stack,
                        line,
                        global_span_offset,
                        *raw_span,
                    );
                let shape = if resolved {
                    &FlatShape::ExternalResolved
                } else {
                    flat_shape
                };
                push_shaped_slice(
                    &mut result.text,
                    shape,
                    &config,
                    line,
                    token_start,
                    token_end,
                );
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
                    let mut style = get_shape_color(flat_shape.as_str(), &config);
                    if highlight {
                        style = get_matching_brackets_style(style, &config);
                    }
                    push_line_slice(
                        &mut result.text,
                        style,
                        line,
                        part.start - global_span_offset,
                        part.end - global_span_offset,
                    );
                }
            }
            _ => push_shaped_slice(
                &mut result.text,
                flat_shape,
                &config,
                line,
                token_start,
                token_end,
            ),
        }
        last_seen_span_end = span.end;
    }

    push_line_slice(
        &mut result.text,
        Style::new(),
        line,
        last_seen_span_end - global_span_offset,
        line.len(),
    );

    result
}

fn push_line_slice(out: &mut StyledText, style: Style, line: &str, start: usize, end: usize) {
    if start < end && end <= line.len() {
        out.push((style, line[start..end].to_string()));
    }
}

fn push_shaped_slice(
    out: &mut StyledText,
    shape: &FlatShape,
    config: &nu_protocol::Config,
    line: &str,
    start: usize,
    end: usize,
) {
    push_line_slice(
        out,
        get_shape_color(shape.as_str(), config),
        line,
        start,
        end,
    );
}

fn get_highlight_block<'a>(
    engine_state: &'a EngineState,
    delta_blocks: &'a [Arc<Block>],
    id: BlockId,
) -> &'a Block {
    let n = engine_state.num_blocks();
    if id.get() < n {
        engine_state.get_block(id).as_ref()
    } else {
        delta_blocks
            .get(id.get() - n)
            .expect("internal error: missing highlight block")
            .as_ref()
    }
}

/// Contents of `span` for `which`: the current line when the span is on it,
/// otherwise engine-state text (aliased externals).
fn span_text<'a>(
    line: &'a str,
    global_span_offset: usize,
    engine_state: &'a EngineState,
    span: Span,
) -> Cow<'a, str> {
    let start = span.start.saturating_sub(global_span_offset);
    let end = span.end.saturating_sub(global_span_offset);
    if span.start >= global_span_offset && end <= line.len() && start <= end {
        Cow::Borrowed(&line[start..end])
    } else {
        String::from_utf8_lossy(engine_state.get_span_contents(span))
    }
}

fn external_is_resolved(
    engine_state: &EngineState,
    stack: &Stack,
    line: &str,
    global_span_offset: usize,
    raw_span: Span,
) -> bool {
    let word = span_text(line, global_span_offset, engine_state, raw_span);
    let paths = env::path_str(engine_state, stack, raw_span).ok();
    let word_os = OsStr::new(word.as_ref());
    let paths_os = paths.as_deref().map(OsStr::new);
    if let Ok(cwd) = engine_state.cwd(Some(stack)) {
        which::which_in(word_os, paths_os, cwd).is_ok()
    } else {
        which::which_in_global(word_os, paths_os)
            .ok()
            .and_then(|mut i| i.next())
            .is_some()
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

fn find_matching_brackets<'a>(
    line: &str,
    get_block: &dyn Fn(BlockId) -> &'a Block,
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
        get_block,
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

fn find_matching_block_end_in_block<'a>(
    line: &str,
    get_block: &dyn Fn(BlockId) -> &'a Block,
    block: &Block,
    global_span_offset: usize,
    global_cursor_offset: usize,
) -> Option<usize> {
    for p in &block.pipelines {
        for e in &p.elements {
            if e.expr.span.contains(global_cursor_offset)
                && let Some(pos) = find_matching_block_end_in_expr(
                    line,
                    get_block,
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
                                get_block,
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

fn find_matching_block_end_in_expr<'a>(
    line: &str,
    get_block: &dyn Fn(BlockId) -> &'a Block,
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
                        get_block,
                        &attr.expr,
                        global_span_offset,
                        global_cursor_offset,
                    )
                })
                .or_else(|| {
                    find_matching_block_end_in_expr(
                        line,
                        get_block,
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
                                get_block,
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
                            get_block,
                            k,
                            global_span_offset,
                            global_cursor_offset,
                        )
                        .or_else(|| {
                            find_matching_block_end_in_expr(
                                line,
                                get_block,
                                v,
                                global_span_offset,
                                global_cursor_offset,
                            )
                        }),
                        RecordItem::Spread(_, record) => find_matching_block_end_in_expr(
                            line,
                            get_block,
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
                        get_block,
                        expr,
                        global_span_offset,
                        global_cursor_offset,
                    )
                })
            }),

            Expr::FullCellPath(b) => find_matching_block_end_in_expr(
                line,
                get_block,
                &b.head,
                global_span_offset,
                global_cursor_offset,
            ),

            Expr::BinaryOp(lhs, op, rhs) => [lhs, op, rhs].into_iter().find_map(|expr| {
                find_matching_block_end_in_expr(
                    line,
                    get_block,
                    expr,
                    global_span_offset,
                    global_cursor_offset,
                )
            }),

            Expr::Collect(_, expr) => find_matching_block_end_in_expr(
                line,
                get_block,
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
                    let nested_block = get_block(*block_id);
                    find_matching_block_end_in_block(
                        line,
                        get_block,
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
                        get_block,
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
                            get_block,
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
    c.len_utf8()
}

#[cfg(test)]
mod tests {
    use super::NuHighlighter;
    use nu_protocol::engine::{EngineState, Stack};
    use reedline::{AbbrExpandContext, Highlighter};
    use rstest::rstest;
    use std::sync::Arc;

    fn make_highlighter() -> NuHighlighter {
        NuHighlighter::new(Arc::new(EngineState::new()), Arc::new(Stack::new()))
    }

    #[test]
    fn highlight_reuses_parse_cache_for_the_same_line() {
        let h = make_highlighter();
        let line = "let x = [1, 2]";
        let first = h.highlight(line, 0);
        let second = h.highlight(line, 0);
        assert_eq!(first.render_simple(), second.render_simple());
        assert_eq!(first.raw_string(), line);

        // Cursor-only change must restyle matching brackets without re-parsing.
        let on_open = h.highlight(line, 8);
        let on_open_again = h.highlight(line, 8);
        assert_eq!(on_open.render_simple(), on_open_again.render_simple());
        assert_eq!(on_open.raw_string(), line);
    }

    #[test]
    fn highlight_slices_cover_the_whole_line() {
        let h = make_highlighter();
        for line in [
            "",
            "ls",
            "use std/iter",
            "1 + 2",
            "{a: 1}",
            "def f [] { 1 }",
        ] {
            let styled = h.highlight(line, line.len());
            assert_eq!(
                styled.raw_string(),
                line,
                "lost text while styling {line:?}"
            );
        }
    }

    #[rstest]
    // 4-byte emoji
    #[case("\"hello 🎉\" hi", 7, false)] // first byte of 🎉
    #[case("\"hello 🎉\" hi", 9, false)] // third byte of 🎉
    #[case("\"hello 🎉\" hi", 13, true)] // after closing quote
    // 8-byte zwj emoji
    #[case("\"hello 🤝🏿\" hi", 9, false)] // inside 🤝
    #[case("\"hello 🤝🏿\" hi", 11, false)] // first byte of 🏿
    #[case("\"hello 🤝🏿\" hi", 13, false)] // inside 🏿
    #[case("\"hello 🤝🏿\" hi", 17, true)] // after closing quote
    // 3-byte unicode
    #[case("\"こんにちは\" hi", 2, false)] // inside こ
    #[case("\"こんにちは\" hi", 5, false)] // inside ん
    #[case("\"こんにちは\" hi", 13, false)] // start of は
    #[case("\"こんにちは\" hi", 18, true)] // after closing quote
    // raw string
    #[case("r#'hello'# hi", 4, false)] // inside 'e'
    #[case("r#'hello'# hi", 11, true)] // after closing #
    // string interpolation
    #[case("$\"hello\" hi", 0, false)] // $ — opening StringInterpolation span (0..2)
    #[case("$\"hello\" hi", 4, false)] // inside literal 'hello'
    #[case("$\"hello\" hi", 9, true)] // after closing quote
    // no string
    #[case("1 + 2", 0, true)]
    #[case("1 + 2", 2, true)]
    // suppress abbreviation expansion in external commands
    #[case("ls -la", 0, true)] // on 'ls'  — FlatShape::External
    #[case("ls -la", 3, false)] // on '-la' — FlatShape::ExternalArg
    #[case("bash -c \"echo hello\"", 0, true)] // on 'bash'            — FlatShape::External
    #[case("bash -c \"echo hello\"", 5, false)] // on '-c'              — FlatShape::ExternalArg
    #[case("bash -c \"echo hello\"", 10, false)] // inside "echo hello"  — FlatShape::ExternalArg
    fn test_should_expand_word_abbr(
        #[case] line: &str,
        #[case] cursor: usize,
        #[case] expected: bool,
    ) {
        let h = make_highlighter();
        assert_eq!(
            h.should_expand_abbr(line, cursor, AbbrExpandContext::WordAbbreviation),
            expected
        );
    }

    #[rstest]
    // bare bang expressions allow expansion
    #[case("!!", 0, true)]
    #[case("!!", 1, true)]
    #[case("!ls", 0, true)]
    #[case("!ls", 2, true)]
    #[case("!-1", 1, true)]
    // bang inside string literals does not suppress expansion
    #[case("\"!!\"", 1, true)]
    #[case("\"!ls\"", 2, true)]
    #[case("r#'!!'#", 3, true)]
    #[case("$\"!!\"", 2, true)]
    // bang as external arg does not suppress expansion
    #[case("bash -c !!", 9, true)]
    #[case("bash -c !ls", 9, true)]
    // bang inside a string that is itself an external arg — shape is ExternalArg, not String.
    // currently there is no way to avoid this
    #[case("echo \"hi !!\"", 9, true)]
    fn test_should_expand_abbr_bang(
        #[case] line: &str,
        #[case] cursor: usize,
        #[case] expected: bool,
    ) {
        let h = make_highlighter();
        assert_eq!(
            h.should_expand_abbr(line, cursor, AbbrExpandContext::BangExpansion),
            expected
        );
    }
}
