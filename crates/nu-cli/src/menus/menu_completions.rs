use crate::completions::{
    CompletionEngine, Returned, SpanClamp, bind_declared_inputs, declared_shape,
    map_value_completions,
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
use std::sync::Arc;

const SELECTION_CHAR: char = '!';

pub struct NuMenuCompleter {
    block_id: BlockId,
    span: Span,
    stack: Stack,
    engine_state: Arc<EngineState>,
    input_mode: InputMode,
}

impl NuMenuCompleter {
    pub fn new(
        block_id: BlockId,
        span: Span,
        stack: Stack,
        engine_state: Arc<EngineState>,
        input_mode: InputMode,
    ) -> Self {
        Self {
            block_id,
            span,
            stack: stack.reset_out_dest().collect_value(),
            engine_state,
            input_mode,
        }
    }
}

impl Completer for NuMenuCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> CompletionResult {
        let parsed = parse_selection_char(line, SELECTION_CHAR);
        let before = parsed.remainder;

        let block = self.engine_state.get_block(self.block_id).clone();
        let replacing = default_span(before, pos, self.input_mode);

        // Pad into the coordinates reedline reads spans in (`before` starts at column
        // `replacing.start` in Diff mode) and read the input at the real cursor.
        let padded = format!("{}{before}", " ".repeat(replacing.start));
        let cursor = padded.floor_char_boundary(pos.min(padded.len()));

        // A menu source is a completer like any other: it opts into what it receives through
        // the positionals it declares, each bound by name. Reedline drives menus on every
        // keystroke, so this parses per keystroke — only when a positional is declared, and
        // then for the cost of one input record.
        let declares_positional = !block.signature.required_positional.is_empty()
            || !block.signature.optional_positional.is_empty();
        if declares_positional {
            let shape = declared_shape(&block.signature);
            let record = CompletionEngine::new(&self.engine_state, &self.stack)
                .completer_input_at(&padded, cursor, shape);
            bind_declared_inputs(&mut self.stack, &block.signature, record);
        }

        let input = Value::nothing(self.span).into_pipeline_data();

        let res = eval_block::<WithoutDebug>(&self.engine_state, &mut self.stack, &block, input)
            .map(|p| p.body);

        let suggestions = match res.and_then(|data| data.into_value(self.span)) {
            Ok(value) => convert_to_suggestions(value, replacing, &padded[..cursor]),
            Err(_) => Vec::new(),
        };

        // Menu sources run synchronously, so results are always final.
        CompletionResult::fresh(suggestions)
    }
}

/// Replacement span when the menu source provides none, matching what reedline feeds the
/// completer in each input mode.
fn default_span(line: &str, pos: usize, input_mode: InputMode) -> reedline::Span {
    match input_mode {
        // `line` is only the text typed since the menu opened; replace it in place.
        InputMode::Diff => reedline::Span {
            start: pos.saturating_sub(line.len()),
            end: pos,
        },
        // Other modes (`InputMode` is non_exhaustive): the suggestion replaces all the
        // text the completer received.
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
        )
    }

    fn values(result: CompletionResult) -> Vec<String> {
        result
            .suggestions()
            .iter()
            .map(|suggestion| suggestion.value.clone())
            .collect()
    }

    /// A source reads the same inputs a completer does, bound by parameter name, so what
    /// used to be `$buffer | split row ' ' | last` is a field of `$token`.
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

    /// Declaring a `contexts` parameter opts a source into the nesting tree, exactly as a
    /// completer does. The cursor is inside the closure, so `each`'s command hangs off the
    /// nesting token.
    #[test]
    fn menu_source_can_declare_contexts() {
        let source = r#"{|contexts| [
            ($contexts.tokens.text | to nuon)
            ($contexts.tokens.1.nested.tokens.text | str join "+")
        ]}"#;

        assert_eq!(
            values(menu(source).complete("each { str tri", 14)),
            ["[each, null]", "str+tri"],
        );
    }

    /// A source returns what a completer returns, so one can serve as either. The `options`
    /// of the record form name engine behaviour a menu has no say in, and are dropped.
    #[rstest]
    #[case::list("{|input| [alpha]}")]
    #[case::envelope("{|input| {completions: [alpha], options: {filter: false}}}")]
    #[case::lone_suggestion("{|input| {value: alpha}}")]
    fn menu_source_accepts_every_completer_shape(#[case] source: &str) {
        assert_eq!(values(menu(source).complete("a", 1)), ["alpha"]);
    }

    /// `null` declines. A menu has no next source to fall through to, so it shows nothing.
    #[test]
    fn menu_source_returning_null_offers_nothing() {
        assert!(values(menu("{|input| null}").complete("a", 1)).is_empty());
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

    /// In `FullBuffer` mode the record describes the site at the cursor, not the buffer tail.
    #[test]
    fn full_buffer_input_describes_the_cursor_not_the_tail() {
        let mut completer = menu("{|token| [$token.text]}");
        completer.input_mode = InputMode::FullBuffer;

        // Cursor after `tri` in `str tri foo`; the token there is `tri`, not the trailing `foo`.
        assert_eq!(values(completer.complete("str tri foo", 7)), ["tri"]);
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
}
