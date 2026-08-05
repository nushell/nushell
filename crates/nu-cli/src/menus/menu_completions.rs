use crate::completions::{
    CompletionEngine, Returned, SpanClamp, declared_shape, map_value_completions,
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

        // A menu source is a completer like any other: same record, same `--full` opt-in.
        // Reedline drives menus on every keystroke, so this parses per keystroke — the cost
        // of one input record rather than three.
        let (shape, full) = declared_shape(&block.signature);
        let input = block
            .signature
            .get_positional(0)
            .and_then(|positional| positional.var_id)
            .map(|var_id| {
                // `replacing.start` is where `before` begins in the line reedline replaces
                // spans in — non-zero only in `InputMode::Diff`, which hands the source just
                // the text typed since the menu opened. Padding to that column costs nothing
                // in the parse (leading whitespace is not a token) and puts every offset in
                // the record in the coordinates a returned `span` is read in.
                let line = format!("{}{before}", " ".repeat(replacing.start));
                let record = CompletionEngine::new(&self.engine_state, &self.stack)
                    .completer_input_at(&line, line.len(), shape);
                (var_id, record)
            });

        if let Some(var_id) = full.and_then(|flag| flag.var_id) {
            self.stack.add_var(var_id, Value::bool(true, self.span));
        }
        if let Some((var_id, record)) = input {
            self.stack.add_var(var_id, record);
        }

        let input = Value::nothing(self.span).into_pipeline_data();

        let res = eval_block::<WithoutDebug>(&self.engine_state, &mut self.stack, &block, input)
            .map(|p| p.body);

        let suggestions = match res.and_then(|data| data.into_value(self.span)) {
            Ok(value) => convert_to_suggestions(value, replacing, pos),
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

/// Read a menu source's output the same way a completer's is read, so one source can serve
/// as either: the same accepted shapes, and the same span/style/description handling and
/// clamping. The `options` a completer may return name engine behaviour a menu has no say
/// in — reedline filters and sorts its own list — so they are read and dropped here.
fn convert_to_suggestions(value: Value, default: reedline::Span, cursor: usize) -> Vec<Suggestion> {
    // `null` declines. A menu has no next source to fall through to, so it shows nothing.
    let Some(returned) = Returned::read(value) else {
        return Vec::new();
    };

    map_value_completions(
        returned.completions.into_iter(),
        default,
        SpanClamp::upto(cursor),
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

    /// A source reads the same `{token, place}` record a completer does, so what used to be
    /// `$buffer | split row ' ' | last` is a field.
    #[test]
    fn menu_source_receives_the_unified_input() {
        let source = r#"{|input| [
            $"token=($input.token.text)"
            $"kind=($input.place.kind)"
            $"target=($input.place.target.start)..($input.place.target.end)"
        ]}"#;

        assert_eq!(
            values(menu(source).complete("str tri", 7)),
            ["token=tri", "kind=positional", "target=4..7"],
        );
    }

    /// `--full` opts a source into the nesting tree, declared exactly as a completer does.
    /// The cursor is inside the closure, so `each`'s command hangs off the nesting token.
    #[test]
    fn menu_source_can_declare_full() {
        let source = r#"{|input, --full| [
            ($input.contexts.tokens.text | to nuon)
            ($input.contexts.tokens.1.nested.tokens.text | str join "+")
            $"full=($full)"
        ]}"#;

        assert_eq!(
            values(menu(source).complete("each { str tri", 14)),
            ["[each, null]", "str+tri", "full=true"],
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
        let mut completer = menu(r#"{|input| [$"($input.place.target.start)"]}"#);
        completer.input_mode = InputMode::Diff;

        // `tri` typed since the menu opened, at column 4 of `str tri`.
        assert_eq!(values(completer.complete("tri", 7)), ["4"]);
    }
}
