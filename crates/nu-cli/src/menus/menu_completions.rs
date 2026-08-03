use crate::completions::{SpanClamp, map_value_completions, menu_input};
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

        let block = self.engine_state.get_block(self.block_id);
        let replacing = default_span(before, pos, self.input_mode);

        // The parse-free input record. Reedline drives menus on every keystroke, so
        // resolving a site each time is wasted on most; one wanting `tokens`/`site` can
        // ask via `commandline complete --input`. `replacing` covers the text reedline
        // handed us, so `before | str substring ...$input.replacing` is exactly what the
        // old `{|buffer, position|}` signature passed as `$buffer`.
        if let Some(var_id) = block
            .signature
            .get_positional(0)
            .and_then(|positional| positional.var_id)
        {
            self.stack
                .add_var(var_id, menu_input(before, replacing, self.span));
        }

        let input = Value::nothing(self.span).into_pipeline_data();

        let res = eval_block::<WithoutDebug>(&self.engine_state, &mut self.stack, block, input)
            .map(|p| p.body);

        let suggestions = match res.and_then(|data| data.into_value(self.span)) {
            Ok(value) => convert_to_suggestions(&value, replacing, pos),
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

/// Read a menu source's output through the shared completer-output conversion, so menus
/// get the same span/style/description handling and clamping as every completer.
fn convert_to_suggestions(
    value: &Value,
    default: reedline::Span,
    cursor: usize,
) -> Vec<Suggestion> {
    let values = match value {
        Value::List { vals, .. } => vals.as_slice(),
        other => std::slice::from_ref(other),
    };

    map_value_completions(values.iter(), default, SpanClamp::upto(cursor))
        .into_iter()
        .map(|semantic| semantic.suggestion)
        .collect()
}
