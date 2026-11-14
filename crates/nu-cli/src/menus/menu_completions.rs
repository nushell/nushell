use crate::{
    completions::{
        CompletionEngine, DeclaredInputs, LegacyInputs, Returned, SpanClamp, bind_declared_inputs,
        catch_completion_panic, map_value_completions, panic_to_shell_error, report,
    },
    menus::{MenuLine, sourced_menu::SourceMode},
};
use nu_engine::eval_block;
use nu_protocol::{
    BlockId, IntoPipelineData, Span, Value,
    debugger::WithoutDebug,
    engine::{EngineState, Stack},
};
use reedline::{Completer, CompletionResult, Suggestion, menu_functions::parse_selection_char};
use std::sync::Arc;

const SELECTION_CHAR: char = '!';

pub struct NuMenuCompleter {
    block_id: BlockId,
    span: Span,
    stack: Stack,
    engine_state: Arc<EngineState>,
    mode: SourceMode,
    line: MenuLine,
}

impl NuMenuCompleter {
    pub fn new(
        block_id: BlockId,
        span: Span,
        stack: Stack,
        engine_state: Arc<EngineState>,
        mode: SourceMode,
        line: MenuLine,
    ) -> Self {
        Self {
            block_id,
            span,
            stack: stack.reset_out_dest().collect_value(),
            engine_state,
            mode,
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
        let mut replacing = default_span(handed, pos, self.mode);
        let (buffer, mut cursor) = self.mode.buffer(&self.line, handed, pos, replacing.start);
        let diff = self.mode.is_diff();
        let original = if diff { buffer.as_str() } else { line };

        // Reedline finds the selector in the text it supplied. Only a Diff
        // fragment starts at an offset into the original buffer.
        let selector = if matches!(self.mode, SourceMode::List { .. }) {
            parsed.marker.and_then(|marker| {
                let input_start = if diff {
                    pos.checked_sub(line.len())?
                } else {
                    0
                };
                let start = input_start + handed.len();
                let end = start + marker.len();
                (original.get(start..end) == Some(marker))
                    .then_some(reedline::Span::new(start, end))
            })
        } else {
            None
        };
        // Live sources already see `handed`. For a Diff source, stop parsing
        // at the selector but use the full changed fragment for the edit.
        let source_buffer = if diff && let Some(selector) = selector {
            replacing = default_span(line, pos, self.mode);
            cursor = cursor.min(selector.start);
            &buffer[..cursor]
        } else {
            &buffer
        };

        // Menu sources opt into inputs; parsed per keystroke only if declared.
        let declares_positional = !block.signature.required_positional.is_empty()
            || !block.signature.optional_positional.is_empty();
        if declares_positional {
            let wanted = DeclaredInputs::from_signature(&block.signature);
            let record = CompletionEngine::new(&self.engine_state, &self.stack).completer_input_at(
                source_buffer,
                cursor,
                wanted,
            );
            bind_declared_inputs(
                &mut self.stack,
                &block.signature,
                record,
                LegacyInputs::menu(&block, source_buffer, cursor, self.span),
            );
        }

        let input = Value::nothing(self.span).into_pipeline_data();

        let res = catch_completion_panic(|| {
            eval_block::<WithoutDebug>(&self.engine_state, &mut self.stack, &block, input)
                .map(|p| p.body)
        })
        .map_err(|payload| panic_to_shell_error(&payload, self.span))
        .and_then(|result| result);

        let mut suggestions = match res.and_then(|data| data.into_value(self.span)) {
            Ok(value) => convert_to_suggestions(value, replacing, &source_buffer[..cursor]),
            Err(err) => {
                report(format!("failed to eval menu source: {err}"));
                Vec::new()
            }
        };

        if let (
            SourceMode::List {
                suggested_span: true,
                ..
            },
            Some(selector),
        ) = (self.mode, selector)
        {
            for suggestion in &mut suggestions {
                include_selector(suggestion, original, selector);
            }
        }

        // Menu sources run synchronously, so results are always final.
        CompletionResult::fresh(suggestions)
    }
}

/// Replacement span when the source names none: what reedline feeds the completer.
fn default_span(line: &str, pos: usize, mode: SourceMode) -> reedline::Span {
    if mode.is_diff() {
        reedline::Span {
            start: pos.saturating_sub(line.len()),
            end: pos,
        }
    } else {
        reedline::Span {
            start: 0,
            end: line.len(),
        }
    }
}

/// Remove the selector even when the source names an earlier replacement span.
/// Preserve any text between the two ranges and apply both changes in one edit.
fn include_selector(suggestion: &mut Suggestion, buffer: &str, selector: reedline::Span) {
    if suggestion.span.end >= selector.end {
        return;
    }
    if let Some(between) = buffer.get(suggestion.span.end..selector.start)
        && !between.is_empty()
    {
        suggestion
            .display_override
            .get_or_insert_with(|| suggestion.value.clone());
        if std::mem::take(&mut suggestion.append_whitespace) {
            suggestion.value.push(' ');
        }
        suggestion.value.push_str(between);
    }
    suggestion.span.end = selector.end;
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
    use crate::menus::{MenuLine, SourcedMenu};
    use nu_parser::parse;
    use nu_protocol::engine::StateWorkingSet;
    use reedline::{
        Editor, InputMode, ListMenu, Menu, MenuBuilder, MenuEvent, OutputMode, UndoBehavior,
    };
    use rstest::rstest;

    /// A menu whose source is `source`, over an engine that knows `nu-command`'s builtins so
    /// a line like `str tri` resolves the way it would at the prompt.
    fn menu(source: &str) -> NuMenuCompleter {
        menu_in(SourceMode::Live, source)
    }

    fn menu_in(mode: SourceMode, source: &str) -> NuMenuCompleter {
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
            mode,
            MenuLine::default(),
        )
    }

    /// The same, on a menu that recorded the line the editor is on -- what a real menu does
    /// on its way to the source. Without it a source sees only the text it was handed.
    fn menu_on(source: &str, line: &str) -> NuMenuCompleter {
        menu_on_in(SourceMode::Live, source, line)
    }

    fn menu_on_in(mode: SourceMode, source: &str, line: &str) -> NuMenuCompleter {
        let completer = menu_in(mode, source);
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

        let mut completer = menu_on_in(SourceMode::Diff, source, "ls ");

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

    fn list_menu(
        source: &str,
        input_mode: InputMode,
        output: OutputMode,
    ) -> (SourcedMenu<ListMenu>, NuMenuCompleter) {
        let mode = SourceMode::List {
            input_mode,
            suggested_span: output == OutputMode::SuggestedSpan,
        };
        let completer = menu_in(mode, source);
        let menu = ListMenu::default()
            .with_input_mode(input_mode)
            .with_output_mode(output);
        let menu = SourcedMenu::new(menu, completer.line.clone(), mode);
        (menu, completer)
    }

    fn set_editor_line(editor: &mut Editor, line: &str) {
        editor.edit_buffer(
            |buffer| {
                buffer.set_buffer(line.to_owned());
                buffer.set_insertion_point(line.len());
            },
            UndoBehavior::CreateUndoPoint,
        );
    }

    #[rstest]
    #[case::diff(InputMode::Diff, false)]
    #[case::cursor_prefix(InputMode::CursorPrefix, true)]
    #[case::full_buffer(InputMode::FullBuffer, true)]
    fn list_keeps_matches_while_typing_a_row_selector(
        #[case] input_mode: InputMode,
        #[case] rows_on_reopen: bool,
    ) {
        let (mut menu, mut completer) = list_menu(
            "{|buffer| [alpha bravo charlie delta] | where $it =~ $buffer | wrap value}",
            input_mode,
            OutputMode::SuggestedSpan,
        );
        let mut editor = Editor::default();
        menu.menu_event(MenuEvent::Activate(false));
        menu.update_values(&mut editor, &mut completer);

        for input in ["!", "!2"] {
            set_editor_line(&mut editor, input);
            menu.menu_event(MenuEvent::Edit(false));
            menu.update_values(&mut editor, &mut completer);

            let rows: Vec<_> = menu
                .get_values()
                .iter()
                .map(|row| row.value.as_str())
                .collect();
            assert_eq!(rows, ["alpha", "bravo", "charlie", "delta"]);
            assert_eq!(editor.get_buffer(), input);
        }

        // Diff treats the existing selector as part of the new starting line.
        menu.menu_event(MenuEvent::Deactivate);
        menu.menu_event(MenuEvent::Activate(false));
        menu.update_values(&mut editor, &mut completer);
        assert_eq!(!menu.get_values().is_empty(), rows_on_reopen);
    }

    #[rstest]
    #[case::earlier_bang("print 'note!2' ", "print 'note!2' doc!2", "print 'note!2' doc")]
    #[case::unicode_and_suffix("cd  | pwd", "cd é!2 | pwd", "cd é")]
    #[case::no_selector("cd ", "cd doc", "cd doc")]
    fn diff_list_source_preserves_context_before_the_selector(
        #[case] opening: &str,
        #[case] edited: &str,
        #[case] expected: &str,
    ) {
        let (mut menu, mut completer) = list_menu(
            r#"{|buffer, position| [$"($position):($buffer)"]}"#,
            InputMode::Diff,
            OutputMode::SuggestedSpan,
        );
        let mut editor = Editor::default();
        set_editor_line(&mut editor, opening);
        menu.menu_event(MenuEvent::Activate(false));
        menu.update_values(&mut editor, &mut completer);

        set_editor_line(&mut editor, edited);
        menu.menu_event(MenuEvent::Edit(false));
        menu.update_values(&mut editor, &mut completer);

        assert_eq!(
            menu.get_values()[0].value,
            format!("{}:{expected}", expected.len())
        );
        assert_eq!(editor.get_buffer(), edited);
    }

    #[rstest]
    #[case::empty_prompt("", "!0", "charlie")]
    #[case::existing_prompt("cd ", "cd !0", "cd charlie")]
    #[case::earlier_selector("cd !2 ", "cd !2 !0", "cd !2 charlie")]
    #[case::unicode_and_suffix("cd  | pwd", "cd é!0 | pwd", "cd charlie | pwd")]
    fn accepting_a_diff_list_row_replaces_the_query_and_selector(
        #[case] opening: &str,
        #[case] edited: &str,
        #[case] expected: &str,
    ) {
        let (mut menu, mut completer) =
            list_menu("{|| [charlie]}", InputMode::Diff, OutputMode::SuggestedSpan);
        let mut editor = Editor::default();
        set_editor_line(&mut editor, opening);
        menu.menu_event(MenuEvent::Activate(false));
        menu.update_values(&mut editor, &mut completer);

        set_editor_line(&mut editor, edited);
        menu.menu_event(MenuEvent::Edit(false));
        menu.update_values(&mut editor, &mut completer);
        menu.replace_in_buffer(&mut editor);
        assert_eq!(editor.get_buffer(), expected);
    }

    #[rstest]
    #[case::cursor_prefix(InputMode::CursorPrefix, "é!0".len())]
    #[case::full_buffer(InputMode::FullBuffer, "é!0".len())]
    #[case::full_buffer_with_cursor_before_selector(InputMode::FullBuffer, "é".len())]
    fn accepting_a_live_list_row_keeps_text_after_the_selector(
        #[case] input_mode: InputMode,
        #[case] cursor: usize,
    ) {
        let (mut menu, mut completer) =
            list_menu("{|| [charlie]}", input_mode, OutputMode::SuggestedSpan);
        let mut editor = Editor::default();
        set_editor_line(&mut editor, " | pwd");
        menu.menu_event(MenuEvent::Activate(false));
        menu.update_values(&mut editor, &mut completer);

        editor.edit_buffer(
            |buffer| {
                buffer.set_buffer("é!0 | pwd".into());
                buffer.set_insertion_point(cursor);
            },
            UndoBehavior::CreateUndoPoint,
        );
        menu.menu_event(MenuEvent::Edit(false));
        menu.update_values(&mut editor, &mut completer);
        menu.replace_in_buffer(&mut editor);
        assert_eq!(editor.get_buffer(), "charlie | pwd");
    }

    #[rstest]
    #[case::suggested_span(OutputMode::SuggestedSpan, "cd charlie /rest ")]
    #[case::extend_to_end(OutputMode::ExtendToEnd, "cd charlie ")]
    #[case::full_buffer(OutputMode::FullBuffer, "charlie ")]
    fn accepting_a_list_row_honors_custom_spans_and_output_modes(
        #[case] output: OutputMode,
        #[case] expected: &str,
    ) {
        for input_mode in [
            InputMode::Diff,
            InputMode::CursorPrefix,
            InputMode::FullBuffer,
        ] {
            let (mut menu, mut completer) = list_menu(
                "{|| [{value: charlie, span: {start: 3, end: 6}, append_whitespace: true}]}",
                input_mode,
                output,
            );
            let mut editor = Editor::default();
            set_editor_line(&mut editor, "cd dir/rest ");
            menu.menu_event(MenuEvent::Activate(false));
            menu.update_values(&mut editor, &mut completer);

            set_editor_line(&mut editor, "cd dir/rest !0");
            menu.menu_event(MenuEvent::Edit(false));
            menu.update_values(&mut editor, &mut completer);

            assert_eq!(menu.get_values()[0].display_value(), "charlie");
            menu.replace_in_buffer(&mut editor);
            assert_eq!(editor.get_buffer(), expected, "input mode {input_mode:?}");
        }
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
        let mut completer = menu_in(SourceMode::Diff, r#"{|place| [$"($place.target.start)"]}"#);

        // `tri` typed since the menu opened, at column 4 of `str tri`.
        assert_eq!(values(completer.complete("tri", 7)), ["4"]);
    }

    /// In `FullBuffer` mode the record still describes the site at the cursor, not the
    /// buffer tail, even though the menu replaces the whole buffer.
    #[test]
    fn full_buffer_input_describes_the_cursor_not_the_tail() {
        let mut completer = menu_on_in(
            SourceMode::Live,
            r#"{|place| [$"($place.kind) ($place.index)"]}"#,
            "str tri foo",
        );

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
        let mut completer = menu_on_in(SourceMode::Live, "{|token| [$token.text]}", "ls -al");
        assert_eq!(values(completer.complete("ls -al", 6)), ["-al"]);
    }
}
