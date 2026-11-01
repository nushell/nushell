//! The REPL's status line: a row under the line being edited that shows its
//! first error, or else the signature of the command under the cursor, the
//! type the pipeline outputs and, for a line of const commands, its value.

use crate::{
    signature_help::{SignatureLabel, find_active_call},
    syntax_highlight::{LineParseCache, ParsedHighlight},
    validation::is_incomplete,
};
use miette::Diagnostic;
use nu_ansi_term::Style;
use nu_color_config::StyleComputer;
use nu_protocol::{
    Config, ParseError, PipelineData, Signals, Span, StatusLineConfig, Type, Value,
    ast::{Block, Expr, Expression, Traverse},
    engine::{EngineState, StateDelta, StateWorkingSet},
    eval_const::eval_const_subexpression,
};
use nu_utils::strip_ansi_unlikely;
use std::{
    ops::ControlFlow,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// How long a paint waits for a const preview before interrupting it.
const PREVIEW_BUDGET: Duration = Duration::from_millis(25);

/// The most graphemes of a preview kept: more than a row of `u16::MAX`
/// columns, the widest a terminal reports, so `render` still decides where
/// the ellipsis goes.
const PREVIEW_GRAPHEMES: usize = u16::MAX as usize;

/// Between the signature, the type and the preview.
const GAP: &str = "   ";

/// Ahead of the status, to set it apart from output.
const INDENT: &str = "  ";

/// The [`reedline::StatusLine`] of the REPL, built for each prompt.
pub(crate) struct NuStatusLine {
    engine_state: Arc<EngineState>,
    parse_cache: LineParseCache,
    config: StatusLineConfig,
    styles: StatusStyles,
    preview: ConstPreview,
}

/// How the parts of the status line are drawn.
pub(crate) struct StatusStyles {
    /// Everything but what the styles below pick out.
    pub(crate) text: Style,
    /// The parameter the cursor is on.
    pub(crate) active: Style,
    /// An error, which takes the whole row.
    pub(crate) error: Style,
}

impl StatusStyles {
    /// The status line drawn in `text`, with the parameter the cursor is on
    /// in bold and underlined, and errors in `error`.
    pub(crate) fn new(text: Style, error: Style) -> Self {
        Self {
            text,
            active: text.bold().underline(),
            error,
        }
    }

    /// The styles `color_config` sets: `status_line`, or `hints`, the style
    /// of history hints, when it doesn't, and `status_line_error`, red unless
    /// set.
    pub(crate) fn from_config(
        config: &Config,
        style_computer: &StyleComputer,
        hints: Style,
    ) -> Self {
        // Like `hints` closures, these only get `null` passed in.
        let nothing = Value::nothing(Span::unknown());
        let text = if config.color_config.contains_key("status_line") {
            style_computer.compute("status_line", &nothing)
        } else {
            hints
        };
        Self::new(text, style_computer.compute("status_line_error", &nothing))
    }
}

impl NuStatusLine {
    /// A status line over `engine_state` that reads the line from the parse
    /// the highlighter keeps in `parse_cache`.
    pub(crate) fn new(
        engine_state: Arc<EngineState>,
        parse_cache: LineParseCache,
        config: StatusLineConfig,
        styles: StatusStyles,
    ) -> Self {
        Self {
            engine_state,
            parse_cache,
            config,
            styles,
            preview: ConstPreview::default(),
        }
    }

    /// The parts of the status for `line` with the cursor at byte `pos`.
    fn segments(&mut self, line: &str, pos: usize) -> Vec<(Style, String)> {
        // reedline keeps `pos` on a char boundary of `line`; a host that
        // doesn't gets no status rather than a panic.
        let Some(before_cursor) = line.get(..pos) else {
            return vec![];
        };
        if line.trim().is_empty() {
            return vec![];
        }
        let parsed = self.parse_cache.parsed(&self.engine_state, line);
        let offset = parsed.global_span_offset;
        let cursor = offset + pos;
        // Where the text typed before the cursor ends, so a cursor resting
        // after a space still belongs to the call or element before it.
        let typed = before_cursor.trim_end().len();
        // A `use` the parse didn't load leaves what it imports unknown, which
        // reads as errors the line doesn't have and changes what it computes.
        let imports_known = !parsed.skipped_module_load;

        if self.config.errors
            && imports_known
            && let Some(error) = first_error(&parsed, offset + typed, cursor)
        {
            return vec![(self.styles.error, format!("✗ {}", error_message(error)))];
        }

        let working_set = parsed.working_set(&self.engine_state);
        let mut segments = vec![];
        // The last character typed before the cursor.
        if let Some(search) = typed.checked_sub(1).map(|i| offset + i) {
            if self.config.signature {
                segments.extend(self.signature(&parsed.block, &working_set, search, cursor));
            }
            if self.config.output_type
                && let Some(ty) = output_type(&parsed.block, &working_set, search)
            {
                push_part(&mut segments, self.styles.text, format!("→ {ty}"));
            }
        }
        if self.config.preview
            && imports_known
            && let Some(value) =
                self.preview
                    .preview(line, &parsed, &working_set, &self.engine_state)
        {
            push_part(&mut segments, self.styles.text, format!("= {value}"));
        }
        segments
    }

    /// The signature of the call around `search`, with the parameter at
    /// `cursor` in the active style.
    fn signature(
        &self,
        block: &Block,
        working_set: &StateWorkingSet,
        search: usize,
        cursor: usize,
    ) -> Vec<(Style, String)> {
        let Some(active) = find_active_call(block, working_set, search, true) else {
            return vec![];
        };
        let signature = working_set.get_decl(active.call.decl_id).signature();
        let label = SignatureLabel::new(&signature);
        let emphasized = if active.in_head && cursor <= active.call.head.end {
            None
        } else if active.in_flag(search) {
            label.flags_part()
        } else {
            label.positional_part(active.positionals_before(cursor))
        };

        let mut segments = vec![];
        for (index, part) in label.parts.into_iter().enumerate() {
            if index > 0 {
                segments.push((self.styles.text, " ".to_string()));
            }
            let style = if Some(index) == emphasized {
                self.styles.active
            } else {
                self.styles.text
            };
            segments.push((style, part));
        }
        segments
    }
}

impl reedline::StatusLine for NuStatusLine {
    fn status(&mut self, line: &str, pos: usize, columns: u16, use_ansi_coloring: bool) -> String {
        let segments = self.segments(line, pos);
        render(&segments, columns, use_ansi_coloring)
    }
}

/// Append `part` after a gap, or first when nothing is there yet.
fn push_part(segments: &mut Vec<(Style, String)>, style: Style, part: String) {
    if !segments.is_empty() {
        segments.push((style, GAP.to_string()));
    }
    segments.push((style, part));
}

/// The error to show: the first one not about the text still being typed.
/// None while the line is unfinished, as the validator judges it.
///
/// An error touching the cursor is in the token being typed. One saying
/// something is missing, found between the end of the typed text (`typed`)
/// and the cursor, is about the token that comes next: `str substring `
/// lacks its range only until it is typed.
fn first_error(parsed: &ParsedHighlight, typed: usize, cursor: usize) -> Option<&ParseError> {
    if is_incomplete(&parsed.errors) {
        return None;
    }
    parsed.errors.iter().find(|error| {
        let span = error.span();
        let at_cursor = span.start <= cursor && cursor <= span.end;
        let next_token = awaits_input(error) && span.start <= cursor && typed <= span.end;
        !(at_cursor || next_token)
    })
}

/// Whether `error` says the line stops short of something it needs.
fn awaits_input(error: &ParseError) -> bool {
    matches!(
        error,
        ParseError::MissingPositional(..)
            | ParseError::KeywordMissingArgument(..)
            | ParseError::MissingFlagParam(..)
            | ParseError::MissingRequiredFlag(..)
            | ParseError::MissingType(..)
            | ParseError::IncompleteMathExpression(..)
            | ParseError::Expected(..)
            | ParseError::ExpectedWithStringMsg(..)
            | ParseError::ExpectedKeyword(..)
    )
}

/// The error's headline and its first label on one line, as `Type mismatch:
/// expected int, found string`. A label that only restates the headline, as
/// `command doesn't support string input` does, is left out.
fn error_message(error: &ParseError) -> String {
    let headline = error.to_string();
    let headline = headline.trim_end_matches('.');
    let label = error
        .labels()
        .and_then(|mut labels| labels.find_map(|label| label.label().map(str::to_owned)))
        .filter(|label| !restates(label, headline));
    match label {
        Some(label) => format!("{headline}: {label}"),
        None => headline.to_string(),
    }
}

/// Whether most of the words of `label` are already in `headline`.
fn restates(label: &str, headline: &str) -> bool {
    let words = |text: &str| -> Vec<String> {
        text.split_whitespace()
            .map(|word| {
                word.trim_matches(|c: char| c.is_ascii_punctuation())
                    .to_lowercase()
            })
            .filter(|word| !word.is_empty())
            .collect()
    };
    let headline = words(headline);
    let label = words(label);
    let shared = label.iter().filter(|word| headline.contains(word)).count();
    label.is_empty() || shared * 4 >= label.len() * 3
}

/// The type of the pipeline element at `pos` (or the last one before it) in
/// the innermost closure, block or subexpression around `pos`, if the parser
/// knows more than `any`.
fn output_type<'a>(
    block: &'a Block,
    working_set: &'a StateWorkingSet,
    pos: usize,
) -> Option<&'a Type> {
    let starts_before = |span: Span| span.start <= pos;
    let pipeline = block.pipelines.iter().rev().find(|pipeline| {
        pipeline
            .elements
            .first()
            .is_some_and(|element| starts_before(element.expr.span))
    })?;
    let element = pipeline
        .elements
        .iter()
        .rev()
        .find(|element| starts_before(element.expr.span))?;
    match element
        .expr
        .find_map(working_set, &|expr| block_around(expr, working_set, pos))
    {
        Some(inner) => output_type(inner, working_set, pos),
        None => Some(&element.expr.ty).filter(|ty| !matches!(ty, Type::Any | Type::Nothing)),
    }
}

/// For [`Traverse::find_map`]: the block `expr` is, a closure, block or
/// subexpression, when `pos` is inside it. On its brackets `pos` is outside,
/// as it is for [`find_active_call`], so a cursor after the `}` of
/// `update name {|r| $r.name | str upcase }` shows the type of `update`.
/// A row condition is not a block here: `where size > 1kb` shows the type
/// of `where`, not `bool`.
fn block_around<'a>(
    expr: &'a Expression,
    working_set: &'a StateWorkingSet,
    pos: usize,
) -> ControlFlow<Option<&'a Block>> {
    if !expr.span.contains(pos) {
        return ControlFlow::Break(None);
    }
    let (Expr::Block(block_id) | Expr::Closure(block_id) | Expr::Subexpression(block_id)) =
        &expr.expr
    else {
        return ControlFlow::Continue(());
    };
    // A block can have no brackets, as the value of `let x = [1 2] | math
    // sum` does, and a block still being typed has no closing one yet.
    let text = working_set.get_span_contents(expr.span);
    let closing = match text.first() {
        Some(b'{') => Some(b'}'),
        Some(b'(') => Some(b')'),
        _ => None,
    };
    let on_bracket = closing.is_some_and(|closing| {
        pos == expr.span.start
            || (text.len() > 1 && text.last() == Some(&closing) && pos + 1 == expr.span.end)
    });
    ControlFlow::Break((!on_bracket).then(|| working_set.get_block(*block_id).as_ref()))
}

/// Draws `segments`, cut to fit in `columns` with an ellipsis, so the status
/// line stays on one row. Styles apply only with `use_ansi_coloring`, and
/// escapes the text carries itself, as a preview of `ansi red` does, are
/// dropped: the row draws its own styles only. Control characters show as
/// [`shown_text`] writes them, so the width is the width of what shows.
fn render(segments: &[(Style, String)], columns: u16, use_ansi_coloring: bool) -> String {
    if segments.is_empty() {
        return String::new();
    }
    let segments: Vec<_> = segments
        .iter()
        .map(|(style, text)| (style, shown_text(text)))
        .collect();
    // The full width: reedline moves the cursor after drawing the row, so text
    // filling it exactly wraps onto nothing.
    let limit = usize::from(columns).saturating_sub(INDENT.len());
    // Measured grapheme by grapheme, as the loop below cuts and reedline
    // draws: the width of a whole string can be less, as for lam-alef.
    let total: usize = segments
        .iter()
        .flat_map(|(_, text)| text.graphemes(true))
        .map(UnicodeWidthStr::width)
        .sum();
    let budget = if total > limit {
        limit.saturating_sub(1)
    } else {
        limit
    };

    let mut out = String::from(INDENT);
    let mut used = 0;
    for (style, text) in segments {
        let mut piece = String::new();
        let mut cut = false;
        for grapheme in text.graphemes(true) {
            let width = grapheme.width();
            if used + width > budget {
                cut = true;
                break;
            }
            used += width;
            piece.push_str(grapheme);
        }
        if cut {
            piece.push('…');
        }
        if use_ansi_coloring {
            out.push_str(&style.paint(piece).to_string());
        } else {
            out.push_str(&piece);
        }
        if cut {
            break;
        }
    }
    out
}

/// `text` as the status line shows it: ANSI escapes dropped, and the other
/// control characters written the way a nushell string escapes them.
fn shown_text(text: &str) -> String {
    strip_ansi_unlikely(&one_line(text)).into_owned()
}

/// `text` on one row: line breaks and tabs shown as `\n`, `\r` and `\t`, and
/// every other control character but `ESC` and `BEL` as its `\u{..}` escape,
/// as `\u{7f}` for DEL. reedline would draw these as `^?` or `M-^E`, wider
/// than `render` measures them. `ESC` and `BEL` are left for
/// [`strip_ansi_unlikely`], which drops the sequences they start and end, as
/// an OSC 8 link ending in `BEL`, and drops a `BEL` of its own.
fn one_line(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\r' => out.push_str("\\r"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\x1b' | '\x07' => out.push(c),
            c if c.is_control() => out.extend(c.escape_unicode()),
            c => out.push(c),
        }
    }
    out
}

/// Evaluates lines built only from const commands, off the REPL thread, so a
/// slow one doesn't stall the keystroke that asked for it. A paint waits
/// `budget` for the value, then interrupts the evaluation: iterating a range
/// stops there, as in `1..10000000000 | math sum`. A command that doesn't
/// check for interrupts keeps running, and no other line is evaluated until
/// it ends.
struct ConstPreview {
    /// How long a paint waits for the value: [`PREVIEW_BUDGET`].
    budget: Duration,
    /// A copy of the REPL's engine state with an interrupt flag of its own,
    /// made on the first preview of the prompt.
    engine_state: Option<Arc<EngineState>>,
    /// The interrupt flag of `engine_state`, set when a paint gives up.
    interrupt: Arc<AtomicBool>,
    /// The thread of the last evaluation, which may still run after the
    /// paint stopped waiting for it.
    worker: Option<JoinHandle<()>>,
    /// The last line asked about and its preview, so moving the cursor
    /// doesn't evaluate the line again.
    last: Option<(String, Option<String>)>,
}

impl Default for ConstPreview {
    fn default() -> Self {
        Self {
            budget: PREVIEW_BUDGET,
            engine_state: None,
            interrupt: Arc::default(),
            worker: None,
            last: None,
        }
    }
}

impl ConstPreview {
    /// The value of `line`, which parsed as `parsed`, when it is built from
    /// const commands and evaluates within the budget.
    fn preview(
        &mut self,
        line: &str,
        parsed: &ParsedHighlight,
        working_set: &StateWorkingSet,
        repl_engine_state: &EngineState,
    ) -> Option<String> {
        if let Some((last_line, preview)) = &self.last
            && last_line == line
        {
            return preview.clone();
        }
        let preview = if parsed.errors.is_empty() && is_previewable(&parsed.block, working_set) {
            if self.is_busy() {
                // An interrupted evaluation still runs: show nothing for now
                // and evaluate this line on a later paint.
                return None;
            }
            self.evaluate(parsed, repl_engine_state)
        } else {
            None
        };
        self.last = Some((line.to_string(), preview.clone()));
        preview
    }

    /// Whether the last evaluation still runs. One that panicked has ended.
    fn is_busy(&self) -> bool {
        self.worker
            .as_ref()
            .is_some_and(|worker| !worker.is_finished())
    }

    /// Evaluates `parsed` on a thread of its own and waits `budget` for the
    /// value, then interrupts the evaluation and gives up on it.
    fn evaluate(
        &mut self,
        parsed: &ParsedHighlight,
        repl_engine_state: &EngineState,
    ) -> Option<String> {
        self.interrupt.store(false, Ordering::Relaxed);
        let engine_state = self
            .engine_state
            .get_or_insert_with(|| {
                let mut engine_state = repl_engine_state.clone();
                engine_state.set_signals(Signals::new(self.interrupt.clone()));
                Arc::new(engine_state)
            })
            .clone();

        let (sender, receiver) = mpsc::channel();
        let block = parsed.block.clone();
        let delta = parsed.delta.clone();
        let worker = thread::Builder::new()
            .name("status line preview".into())
            .spawn(move || {
                // The paint that asked may have stopped waiting.
                let _ = sender.send(evaluate_const_line(&engine_state, &block, &delta));
            })
            .ok()?;
        self.worker = Some(worker);

        match receiver.recv_timeout(self.budget) {
            Ok(preview) => preview,
            Err(_) => {
                self.interrupt.store(true, Ordering::Relaxed);
                None
            }
        }
    }
}

/// The value of `block` as the status line shows it, or `None` when it
/// isn't a constant or has nothing to show. The text is [`shown_text`], cut
/// to [`PREVIEW_GRAPHEMES`], so a huge value costs a paint no more than one
/// that fills the row.
fn evaluate_const_line(
    engine_state: &EngineState,
    block: &Block,
    delta: &StateDelta,
) -> Option<String> {
    let mut working_set = StateWorkingSet::new(engine_state);
    working_set.delta = delta.clone();
    let span = block.span.unwrap_or_else(Span::unknown);
    let value = eval_const_subexpression(&working_set, block, PipelineData::empty(), span)
        .and_then(|data| data.into_value(span))
        .ok()?;
    if matches!(value, Value::Nothing { .. } | Value::Error { .. }) {
        return None;
    }
    let mut text = shown_text(&value.to_expanded_string(", ", engine_state.get_config()));
    if let Some((end, _)) = text.grapheme_indices(true).nth(PREVIEW_GRAPHEMES) {
        text.truncate(end);
    }
    Some(text)
}

/// Whether every command in `block` is const, so it can be evaluated while
/// it is typed, and it computes something: a literal would preview as itself.
fn is_previewable(block: &Block, working_set: &StateWorkingSet) -> bool {
    #[derive(PartialEq)]
    enum Part {
        Computed,
        NotConst,
    }
    let mut parts = vec![];
    block.flat_map(
        working_set,
        &|expr| match &expr.expr {
            Expr::Call(call) if working_set.get_decl(call.decl_id).is_const() => {
                vec![Part::Computed]
            }
            Expr::Call(_)
            | Expr::ExternalCall(..)
            | Expr::Closure(_)
            | Expr::RowCondition(_)
            | Expr::Collect(..)
            | Expr::Overlay(_) => vec![Part::NotConst],
            Expr::BinaryOp(..) | Expr::UnaryNot(_) | Expr::Var(_) => vec![Part::Computed],
            Expr::FullCellPath(path) if !path.tail.is_empty() => vec![Part::Computed],
            _ => vec![],
        },
        &mut parts,
    );
    let redirected = block
        .pipelines
        .iter()
        .flat_map(|pipeline| &pipeline.elements)
        .any(|element| element.redirection.is_some());
    !redirected && parts.contains(&Part::Computed) && !parts.contains(&Part::NotConst)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_ansi_term::Color;
    use nu_protocol::{engine::Stack, record};
    use rstest::rstest;

    /// A status line over the default commands, with every part on, colored
    /// hints and a preview that waits as long as the evaluation takes.
    fn status_line(config: StatusLineConfig) -> NuStatusLine {
        let engine_state =
            nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
        let mut status_line = NuStatusLine::new(
            Arc::new(engine_state),
            LineParseCache::default(),
            config,
            StatusStyles::new(
                Style::new().fg(Color::DarkGray),
                Style::new().fg(Color::Red),
            ),
        );
        status_line.preview.budget = Duration::from_secs(60);
        status_line
    }

    fn only(part: &str) -> StatusLineConfig {
        StatusLineConfig {
            errors: part == "errors",
            signature: part == "signature",
            output_type: part == "type",
            preview: part == "preview",
            ..StatusLineConfig::default()
        }
    }

    /// The status for `line` with the cursor at its end, as text.
    fn status(config: StatusLineConfig, line: &str) -> String {
        shown(&mut status_line(config), line)
    }

    /// What `status_line` shows for `line` with the cursor at its end.
    fn shown(status_line: &mut NuStatusLine, line: &str) -> String {
        shown_at(status_line, line, line.len())
    }

    /// What `status_line` shows for `line` with the cursor at byte `pos`.
    fn shown_at(status_line: &mut NuStatusLine, line: &str, pos: usize) -> String {
        let segments = status_line.segments(line, pos);
        segments.into_iter().map(|(_, text)| text).collect()
    }

    #[rstest]
    #[case::after_the_name("str substring ", Some("<range>"))]
    #[case::typing_the_argument("str substring 1..2", Some("<range>"))]
    #[case::after_the_argument("str substring 1..2 ", Some("...(rest)"))]
    #[case::in_a_flag("ls --all", Some("{flags}"))]
    #[case::on_the_name("ls", None)]
    fn signature_marks_the_parameter_being_typed(#[case] line: &str, #[case] active: Option<&str>) {
        let mut status_line = status_line(only("signature"));
        let segments = status_line.segments(line, line.len());

        let marked = segments
            .iter()
            .find(|(style, _)| *style == status_line.styles.active)
            .map(|(_, text)| text.as_str());
        assert_eq!(marked, active);
    }

    #[test]
    fn signature_of_a_command_the_line_defines() {
        assert_eq!(
            status(only("signature"), "def foo [a: int, b?: string] {}; foo 1 "),
            "foo {flags} <a> (b)"
        );
    }

    #[rstest]
    #[case::still_typing_the_token(r#"let x: int = "a""#, "")]
    #[case::token_finished(r#"let x: int = "a" "#, "✗ Type mismatch: expected int, found string")]
    #[case::argument_not_typed_yet("str substring ", "")]
    #[case::argument_skipped(
        "str substring | str length",
        "✗ Missing required positional argument: missing range"
    )]
    #[case::label_restates_the_headline(
        "[1 2] | str downcase | ls",
        "✗ Command does not support list<int> input"
    )]
    #[case::unfinished_block("if true {", "")]
    fn errors_wait_for_the_text_being_typed(#[case] line: &str, #[case] expected: &str) {
        assert_eq!(status(only("errors"), line), expected);
    }

    #[rstest]
    #[case::pipeline_end("[1 2 3] | math sum", "→ number")]
    #[case::after_a_pipe("ls | ", "→ table")]
    #[case::unknown("ls | get name", "")]
    fn type_of_the_pipeline_at_the_cursor(#[case] line: &str, #[case] expected: &str) {
        assert_eq!(status(only("type"), line), expected);
    }

    /// The type comes from the block the cursor is in, or on its closing
    /// bracket from the block around it, as the signature does. `before` is
    /// the text before the cursor.
    #[rstest]
    #[case::in_a_closure("ls | update name {|r| [1 2] | math sum", " }", "→ number")]
    #[case::in_an_unclosed_closure("ls | update name {|r| [1 2] | math sum", "", "→ number")]
    #[case::after_a_pipe_in_a_closure("ls | update name {|r| [1 2] | ", "}", "→ list<int>")]
    #[case::after_the_closure(
        "ls | update name {|r| [1 2] | math sum }",
        "",
        "→ oneof<table, list<any>>"
    )]
    #[case::in_a_subexpression("([1 2] | math sum", " | into string)", "→ number")]
    #[case::value_of_a_let("let x = [1 2] | math sum", "", "→ number")]
    fn type_of_the_innermost_pipeline(
        #[case] before: &str,
        #[case] after: &str,
        #[case] expected: &str,
    ) {
        let line = format!("{before}{after}");
        let mut status_line = status_line(only("type"));
        assert_eq!(shown_at(&mut status_line, &line, before.len()), expected);
    }

    #[rstest]
    #[case::inside_a_char("é", 1)]
    #[case::past_the_end("ls", 3)]
    fn cursor_off_the_line_shows_nothing(#[case] line: &str, #[case] pos: usize) {
        let mut status_line = status_line(StatusLineConfig::default());
        assert_eq!(shown_at(&mut status_line, line, pos), "");
    }

    #[rstest]
    #[case::const_commands(r#""a,b" | split row ",""#, "= [a, b]")]
    #[case::arithmetic("1 + 2", "= 3")]
    #[case::control_characters(r#""a\t\u{85}b" | str trim"#, r"= a\t\u{85}b")]
    #[case::link_ending_in_bel(r#""\e]8;;http://a\u{7}" + "link""#, "= link")]
    #[case::stray_bel(r#""a\u{7}b" | str trim"#, "= ab")]
    #[case::literal(r#""abc""#, "")]
    #[case::not_const("ls | length", "")]
    fn preview_of_a_const_line(#[case] line: &str, #[case] expected: &str) {
        assert_eq!(status(only("preview"), line), expected);
    }

    #[test]
    fn preview_past_its_budget_is_not_shown() {
        let mut status_line = status_line(only("preview"));
        status_line.preview.budget = Duration::ZERO;

        assert_eq!(shown(&mut status_line, "1..1000000 | math sum"), "");
        if let Some(worker) = status_line.preview.worker.take() {
            worker.join().expect("the preview doesn't panic");
        }
    }

    #[test]
    fn interrupted_preview_has_no_value() {
        let mut engine_state =
            nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
        engine_state.set_signals(Signals::new(Arc::new(AtomicBool::new(true))));
        let parsed = LineParseCache::default().parsed(&engine_state, "1..1000000 | math sum");

        let preview = evaluate_const_line(&engine_state, &parsed.block, &parsed.delta);
        assert_eq!(preview, None);
    }

    /// `math abs` returns a stream at once, so the interrupt lands while the
    /// const collects it.
    #[test]
    fn const_interrupted_while_collected_is_an_error() {
        let mut engine_state =
            nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
        let interrupt = Arc::new(AtomicBool::new(false));
        engine_state.set_signals(Signals::new(interrupt.clone()));
        let interrupter = thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            interrupt.store(true, Ordering::Relaxed);
        });

        let line = "const xs = 1..100000000000 | math abs";
        let parsed = LineParseCache::default().parsed(&engine_state, line);
        interrupter.join().expect("the interrupter doesn't panic");
        assert!(
            parsed
                .errors
                .iter()
                .any(|error| error.to_string().contains("interrupted")),
            "{:?}",
            parsed.errors
        );
    }

    #[test]
    fn line_asked_about_while_busy_is_evaluated_later() {
        let mut status_line = status_line(only("preview"));
        let (release, wait) = mpsc::channel::<()>();
        status_line.preview.worker = Some(thread::spawn(move || {
            let _ = wait.recv();
        }));

        assert_eq!(shown(&mut status_line, "1 + 2"), "");
        drop(release);
        if let Some(worker) = status_line.preview.worker.take() {
            worker.join().expect("the stand-in worker doesn't panic");
        }
        assert_eq!(shown(&mut status_line, "1 + 2"), "= 3");
    }

    #[test]
    fn preview_is_cut_past_any_row() {
        // `[1, 2, 3, …, 20000]` is over 120,000 characters.
        let preview = status(only("preview"), "1..20000 | math abs");
        assert_eq!(
            preview.graphemes(true).count(),
            "= ".len() + PREVIEW_GRAPHEMES
        );
    }

    #[test]
    fn a_use_not_loaded_hides_errors() {
        // `$X` comes from the module, which the parse doesn't load.
        assert_eq!(status(only("errors"), "use ./consts.nu *; $X + 1"), "");
        assert_eq!(status(only("errors"), "$X + 1"), "✗ Variable not found");
    }

    #[test]
    fn disabled_parts_show_nothing() {
        let config = StatusLineConfig {
            errors: false,
            signature: false,
            output_type: false,
            preview: false,
            ..StatusLineConfig::default()
        };
        assert_eq!(status(config, "[1 2 3] | math sum"), "");
    }

    #[test]
    fn render_cuts_to_the_width_with_an_ellipsis() {
        let segments = vec![(Style::new(), "abcdefghij".to_string())];

        assert_eq!(render(&segments, 20, false), "  abcdefghij");
        assert_eq!(render(&segments, 12, false), "  abcdefghij");
        assert_eq!(render(&segments, 10, false), "  abcdefg…");
    }

    #[test]
    fn render_drops_escapes_in_the_text() {
        let style = Style::new().fg(Color::DarkGray);
        let segments = vec![(style, "\x1b[31mhot\x1b[0m".to_string())];

        assert_eq!(render(&segments, 20, false), "  hot");
        assert_eq!(
            render(&segments, 20, true),
            format!("  {}", style.paint("hot"))
        );
        assert_eq!(render(&segments, 4, false), "  h…");
        // The escapes take no room: `hot` fills the row exactly.
        assert_eq!(render(&segments, 5, false), "  hot");
    }

    #[test]
    fn render_measures_control_characters_as_written() {
        let segments = vec![(Style::new(), "a\u{7f}\u{85}\tb\r\n".to_string())];

        // `a\u{7f}\u{85}\tb\r\n` fills 20 columns, so a row of 22 exactly.
        assert_eq!(render(&segments, 22, false), r"  a\u{7f}\u{85}\tb\r\n");
        assert_eq!(render(&segments, 21, false), r"  a\u{7f}\u{85}\tb\r…");
    }

    #[test]
    fn render_measures_graphemes_as_it_cuts() {
        // Lam-alef is one column as a string, two as graphemes.
        let lam_alef = "\u{644}\u{627}";
        assert_eq!(lam_alef.width(), 1);
        let segments = vec![(Style::new(), lam_alef.repeat(2))];

        assert_eq!(render(&segments, 5, false), format!("  {lam_alef}…"));
    }

    #[rstest]
    #[case::error(
        "errors",
        r#"let x: int = "a" "#,
        "✗ Type mismatch: expected int, found string"
    )]
    #[case::signature(
        "signature",
        "str substring ",
        "str substring {flags} <range> ...(rest)"
    )]
    #[case::preview_of_colored_text("preview", r#"(ansi red) + "hot""#, "= hot")]
    fn colors_follow_use_ansi_coloring(
        #[case] part: &str,
        #[case] line: &str,
        #[case] plain: &str,
    ) {
        let mut status_line = status_line(only(part));
        let mut draw = |use_ansi_coloring| {
            reedline::StatusLine::status(&mut status_line, line, line.len(), 100, use_ansi_coloring)
        };

        assert_eq!(draw(false), format!("  {plain}"));
        let colored = draw(true);
        assert!(colored.contains('\x1b'));
        assert_eq!(strip_ansi_unlikely(&colored), format!("  {plain}"));
    }

    /// The styles of a `color_config` holding only `entries`, as a theme
    /// that replaces it does, with history hints in cyan.
    fn styles_from(entries: &[(&str, Value)]) -> StatusStyles {
        let config = Config {
            color_config: entries
                .iter()
                .map(|(key, value)| (key.to_string(), value.clone()))
                .collect(),
            ..Config::default()
        };
        let mut engine_state = EngineState::new();
        engine_state.set_config(config);
        let stack = Stack::new();
        let style_computer = StyleComputer::from_config(&engine_state, &stack);
        let hints = Style::new().fg(Color::Cyan);
        StatusStyles::from_config(engine_state.get_config(), &style_computer, hints)
    }

    #[test]
    fn styles_come_from_color_config() {
        let error = record! {
            "fg" => Value::test_string("yellow"),
            "attr" => Value::test_string("b"),
        };
        let styles = styles_from(&[
            ("status_line", Value::test_string("blue")),
            ("status_line_error", Value::test_record(error)),
        ]);

        assert_eq!(styles.text, Style::new().fg(Color::Blue));
        assert_eq!(
            styles.active,
            Style::new().fg(Color::Blue).bold().underline()
        );
        assert_eq!(styles.error, Style::new().fg(Color::Yellow).bold());
    }

    #[test]
    fn styles_not_set_are_the_hints_and_red() {
        let styles = styles_from(&[]);

        assert_eq!(styles.text, Style::new().fg(Color::Cyan));
        assert_eq!(
            styles.active,
            Style::new().fg(Color::Cyan).bold().underline()
        );
        assert_eq!(styles.error, Style::new().fg(Color::Red));
    }

    #[rstest]
    #[case(
        "command doesn't support string input",
        "Command does not support string input",
        true
    )]
    #[case("expected int, found string", "Type mismatch", false)]
    #[case("missing range", "Missing required positional argument", false)]
    fn restated_labels(#[case] label: &str, #[case] headline: &str, #[case] expected: bool) {
        assert_eq!(restates(label, headline), expected);
    }
}
