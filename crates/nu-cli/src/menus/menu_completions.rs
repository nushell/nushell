use crate::{
    completions::{
        CompletionEngine, DeclaredInputs, LegacyInputs, Returned, SpanClamp, bind_declared_inputs,
        map_value_completions, panic_to_shell_error, report,
    },
    menus::MenuLine,
};
use nu_engine::eval_block;
use nu_protocol::{
    BlockId, IntoPipelineData, Span, Value,
    debugger::WithoutDebug,
    engine::{EngineState, Stack},
};
use reedline::{
    Completer, CompletionResult, InputMode, Suggestion, menu_functions::parse_selection_char,
};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

const SELECTION_CHAR: char = '!';

pub struct NuMenuCompleter {
    block_id: BlockId,
    span: Span,
    stack: Stack,
    engine_state: Arc<EngineState>,
    input_mode: InputMode,
    /// The line the menu is on, which reedline hands the menu rather than its source.
    line: MenuLine,
}

impl NuMenuCompleter {
    pub fn new(
        block_id: BlockId,
        span: Span,
        stack: Stack,
        engine_state: Arc<EngineState>,
        input_mode: InputMode,
        line: MenuLine,
    ) -> Self {
        Self {
            block_id,
            span,
            stack: stack.reset_out_dest().collect_value(),
            engine_state,
            input_mode,
            line,
        }
    }
}

impl Completer for NuMenuCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> CompletionResult {
        let parsed = parse_selection_char(line, SELECTION_CHAR);
        let handed = parsed.remainder;

        let block = self.engine_state.get_block(self.block_id).clone();
        // Reedline replacement range — never fed into `$token`/`$place`.
        let replacing = default_span(handed, pos, self.input_mode);

        // Menu buffer from recorded line or padded handed text.
        let buffer = self
            .line
            .read()
            .unwrap_or_else(|| " ".repeat(replacing.start) + handed);
        let cursor = buffer.floor_char_boundary(pos.min(buffer.len()));

        // Menu sources opt into inputs; parsed per keystroke only if declared.
        let declares_positional = !block.signature.required_positional.is_empty()
            || !block.signature.optional_positional.is_empty();
        if declares_positional {
            let wanted = DeclaredInputs::from_signature(&block.signature);
            let record = CompletionEngine::new(&self.engine_state, &self.stack)
                .completer_input_at(&buffer, cursor, wanted);
            bind_declared_inputs(
                &mut self.stack,
                &block.signature,
                record,
                LegacyInputs::menu(&block, handed, pos, self.span),
            );
        }

        let input = Value::nothing(self.span).into_pipeline_data();

        let res = catch_unwind(AssertUnwindSafe(|| {
            eval_block::<WithoutDebug>(&self.engine_state, &mut self.stack, &block, input)
                .map(|p| p.body)
        }))
        .map_err(|payload| panic_to_shell_error(&payload, self.span))
        .and_then(|result| result);

        let suggestions = match res.and_then(|data| data.into_value(self.span)) {
            Ok(value) => convert_to_suggestions(value, replacing, &buffer[..cursor]),
            Err(err) => {
                report(format!("failed to eval menu source: {err}"));
                Vec::new()
            }
        };

        // Menu sources run synchronously, so results are always final.
        CompletionResult::fresh(suggestions)
    }
}

/// Replacement span when the source names none: what reedline feeds the completer.
fn default_span(line: &str, pos: usize, input_mode: InputMode) -> reedline::Span {
    match input_mode {
        // `line` is only text typed since the menu opened; replace it in place.
        InputMode::Diff => reedline::Span {
            start: pos.saturating_sub(line.len()),
            end: pos,
        },
        // Other (non_exhaustive) modes replace all handed text.
        _ => reedline::Span {
            start: 0,
            end: line.len(),
        },
    }
}

/// Read a menu source's output the way a completer's is read: same accepted shapes and span
/// handling, clamped against `seen`. The `options` of the record form are dropped — reedline
/// filters and sorts its own list.
fn convert_to_suggestions(value: Value, default: reedline::Span, seen: &str) -> Vec<Suggestion> {
    // `null` declines. A menu has no next source to fall through to, so it shows nothing.
    let Some(returned) = Returned::read(value) else {
        return Vec::new();
    };

    map_value_completions(
        returned.completions.into_iter(),
        default,
        SpanClamp::within(seen),
    )
    .into_iter()
    .map(|semantic| semantic.suggestion)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_parser::parse;
    use nu_protocol::engine::StateWorkingSet;
    use rstest::rstest;

    /// A menu whose source is `source`, over an engine that knows `nu-command`'s builtins so
    /// a line like `str tri` resolves the way it would at the prompt.
    fn menu(source: &str) -> NuMenuCompleter {
        let mut engine_state =
            nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());

        let block_id = {
            let mut working_set = StateWorkingSet::new(&engine_state);
            let block = parse(&mut working_set, None, source.as_bytes(), false);
            let closure = block
                .pipelines
                .first()
                .and_then(|pipeline| pipeline.elements.first())
                .expect("the source parses to one closure");

            let nu_protocol::ast::Expr::Closure(block_id) = closure.expr.expr else {
                panic!("the source is a closure");
            };

            let delta = working_set.render();
            engine_state
                .merge_delta(delta)
                .expect("the parsed closure merges");
            block_id
        };

        NuMenuCompleter::new(
            block_id,
            Span::unknown(),
            Stack::new(),
            Arc::new(engine_state),
            InputMode::CursorPrefix,
            MenuLine::default(),
        )
    }

    /// The same, on a menu that recorded the line the editor is on -- what a real menu does
    /// on its way to the source. Without it a source sees only the text it was handed.
    fn menu_on(source: &str, line: &str) -> NuMenuCompleter {
        let completer = menu(source);
        completer.line.record(line);
        completer
    }

    fn values(result: CompletionResult) -> Vec<String> {
        result
            .suggestions()
            .iter()
            .map(|suggestion| suggestion.value.clone())
            .collect()
    }

    /// A source reads the same inputs a completer / `commandline complete --input` does:
    /// `token` is the completion site at the cursor (not the whole buffer), even when the
    /// menu's reedline replacement range is the whole handed line.
    #[test]
    fn menu_source_receives_the_unified_input() {
        let source = r#"{|token, place| [
            $"token=($token.text)"
            $"kind=($place.kind)"
            $"target=($place.target.start)..($place.target.end)"
        ]}"#;

        assert_eq!(
            values(menu(source).complete("str tri", 7)),
            ["token=tri", "kind=positional", "target=4..7"],
        );
    }

    /// The line a `Diff` menu is on reaches its source, though reedline hands the source
    /// only what was typed since the menu opened: opening one on `ls ` completes `ls`'s
    /// first argument, not a command on a blank line.
    #[test]
    fn menu_source_sees_the_line_the_menu_is_on() {
        let source = r#"{|token, place, buffer| [
            $"buffer=($buffer)"
            $"kind=($place.kind)"
            $"token=($token.kind)"
            $"target=($place.target.start)..($place.target.end)"
        ]}"#;

        let mut completer = menu_on(source, "ls ");
        completer.input_mode = InputMode::Diff;

        assert_eq!(
            values(completer.complete("", 3)),
            [
                "buffer=ls ",
                "kind=positional",
                "token=value",
                "target=3..3"
            ],
        );
    }

    /// Declaring a `buffer` parameter opts a source into the whole line up to the cursor,
    /// exactly as a completer does.
    #[test]
    fn menu_source_can_declare_buffer() {
        let source = "{|buffer| [$buffer]}";

        assert_eq!(
            values(menu(source).complete("each { str tri", 14)),
            ["each { str tri"],
        );
    }

    /// A source returns what a completer returns, so one can serve as either. The `options`
    /// of the record form name engine behaviour a menu has no say in, and are dropped.
    #[rstest]
    #[case::list("{|input| [alpha]}")]
    #[case::envelope("{|input| {completions: [alpha], options: {filter: false}}}")]
    #[case::lone_suggestion("{|input| {value: alpha}}")]
    #[case::bare_value("{|input| 'alpha'}")]
    fn menu_source_accepts_every_completer_shape(#[case] source: &str) {
        assert_eq!(values(menu(source).complete("a", 1)), ["alpha"]);
    }

    /// `null` declines. A menu has no next source to fall through to, so it shows nothing.
    #[test]
    fn menu_source_returning_null_offers_nothing() {
        assert!(values(menu("{|input| null}").complete("a", 1)).is_empty());
    }

    #[test]
    fn panicking_menu_source_returns_no_values() {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            values(menu("{|input| panic 'boom'}").complete("a", 1))
        }));

        assert_eq!(
            result.expect("menu source panic must be contained"),
            Vec::<String>::new()
        );
    }

    /// In `Diff` mode reedline hands the source only the text typed since the menu opened,
    /// while still replacing spans in the whole line. The record's offsets must be in that
    /// same coordinate system, or a source reading `place.target` to build its own `span`
    /// would replace the wrong bytes.
    #[test]
    fn menu_input_offsets_match_the_span_reedline_replaces() {
        let mut completer = menu(r#"{|place| [$"($place.target.start)"]}"#);
        completer.input_mode = InputMode::Diff;

        // `tri` typed since the menu opened, at column 4 of `str tri`.
        assert_eq!(values(completer.complete("tri", 7)), ["4"]);
    }

    /// In `FullBuffer` mode the record still describes the site at the cursor, not the
    /// buffer tail, even though the menu replaces the whole buffer.
    #[test]
    fn full_buffer_input_describes_the_cursor_not_the_tail() {
        let mut completer = menu_on(
            r#"{|place| [$"($place.kind) ($place.index)"]}"#,
            "str tri foo",
        );
        completer.input_mode = InputMode::FullBuffer;

        // Cursor after `tri` in `str tri foo`: `str`'s first argument, not the trailing `foo`.
        assert_eq!(
            values(completer.complete("str tri foo", 7)),
            ["positional 0"]
        );
    }

    /// Suggestion spans are clamped against the seen text and floored to a char boundary (#5127).
    #[test]
    fn menu_suggestion_span_is_clamped_to_a_char_boundary() {
        // `é` occupies bytes 4..6 of `str é`, so 5 is mid-character and 99 is past the end.
        let mut completer = menu("{|place| [{value: v, span: {start: 5, end: 99}}]}");
        let result = completer.complete("str é", "str é".len());
        let suggestions = result.suggestions();

        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].span, reedline::Span::new(4, 6));
    }

    /// Legacy compat: a source declaring the old `{|buffer, position|}` positionals
    /// receives the line text and the cursor, exactly as before.
    #[test]
    fn menu_source_receives_legacy_buffer_and_position() {
        let source = "{|buffer, position| [$buffer, $\"pos=($position)\"]}";
        assert_eq!(
            values(menu(source).complete("str tri", 7)),
            ["str tri", "pos=7"],
        );
    }
    /// Discord: `ls -al` at the final `l` must hand the flag token, not the whole buffer.
    #[test]
    fn menu_token_for_a_flag_matches_commandline_complete() {
        let source = r#"{|token| [
            $"text=($token.text)"
            $"start=($token.span.start)"
            $"kind=($token.kind)"
        ]}"#;
        assert_eq!(
            values(menu_on(source, "ls -al").complete("ls -al", 6)),
            ["text=-al", "start=3", "kind=flag"],
        );
    }

    /// Discord: inside `each {|r|` the token is the block, not the whole pipeline.
    #[test]
    fn menu_token_inside_a_block_is_not_the_whole_buffer() {
        let line = "ls | each {|r|";
        let source = r#"{|token, buffer| [
            $"text=($token.text)"
            $"start=($token.span.start)"
            $"buffer=($buffer)"
        ]}"#;
        assert_eq!(
            values(menu_on(source, line).complete(line, line.len())),
            vec![
                "text={|r|".into(),
                "start=10".into(),
                format!("buffer={line}"),
            ],
        );
    }

    /// Discord: at the end of `… | str uppercase` the site token is the multi-word
    /// command head, not the whole buffer / pipeline.
    #[test]
    fn menu_token_for_str_uppercase_pipeline_is_the_site() {
        let line = "ls | each {|r| $r.name | str uppercase";
        let source = r#"{|token, buffer| [
            $"text=($token.text)"
            $"start=($token.span.start)"
            $"kind=($token.kind)"
            $"buffer=($buffer)"
        ]}"#;
        assert_eq!(
            values(menu_on(source, line).complete(line, line.len())),
            vec![
                "text=str uppercase".into(),
                "start=25".into(),
                "kind=head".into(),
                format!("buffer={line}"),
            ],
        );
    }

    /// Menu `$token` / `$place` match `commandline complete --input` / `completer_input_at`
    /// for the Discord + HackMD adversarial lines (parity across the two surfaces).
    #[test]
    fn menu_token_matches_completer_input_at_for_adversarial_lines() {
        let source = r#"{|token, place| [
            $"text=($token.text)"
            $"start=($token.span.start)"
            $"kind=($place.kind)"
            $"index=($place.index? | default 'none')"
        ]}"#;

        let cases = [
            (
                "ls -al",
                ["text=-al", "start=3", "kind=flag-name", "index=none"],
            ),
            (
                "ls | where size >",
                ["text=>", "start=16", "kind=operator", "index=none"],
            ),
            (
                "ls | where size > 1",
                ["text=1", "start=18", "kind=positional", "index=0"],
            ),
            (
                "ls | where size > ",
                ["text=", "start=18", "kind=positional", "index=0"],
            ),
            (
                "ls | each {|r|",
                ["text={|r|", "start=10", "kind=positional", "index=0"],
            ),
            (
                "ls | each {|r| $r.name | str uppercase",
                [
                    "text=str uppercase",
                    "start=25",
                    "kind=command",
                    "index=none",
                ],
            ),
        ];

        for (line, expected) in cases {
            assert_eq!(
                values(menu_on(source, line).complete(line, line.len())),
                expected,
                "menu token/place for {line:?}"
            );
        }
    }

    /// `only_buffer_difference: false` (CursorPrefix) still resolves the site token.
    #[test]
    fn cursor_prefix_menu_token_is_the_site_not_the_line() {
        let mut completer = menu_on("{|token| [$token.text]}", "ls -al");
        completer.input_mode = InputMode::CursorPrefix;
        assert_eq!(values(completer.complete("ls -al", 6)), ["-al"]);
    }
}
