//! The one closure contract shared by every hook: `tui bind`, menu actions,
//! `tui button`, `--on-select`, and the `tui run` refresh closure.
//!
//! A hook receives the current state record (the same shape `tui run`
//! returns) as `$in`, and as its first parameter when it declares one. What
//! it returns decides what happens:
//!
//! - `null` (or no output): nothing changes.
//! - a record with an `action` field: `{action: submit, selected: ...}` ends
//!   the TUI with that value; `{action: quit}` quits.
//! - anything else: it replaces the shared data list, so every table, log,
//!   and tree that shows the outer pipeline's data redraws.

use nu_engine::ClosureEvalOnce;
use nu_protocol::engine::{Closure, EngineState, Stack};
use nu_protocol::{IntoPipelineData, PipelineData, Span, Value};

#[derive(Debug, Clone)]
pub enum HookOutcome {
    Nothing,
    Data(Value),
    Submit(Value),
    Quit,
    Error(String),
}

/// Whether the closure declares a positional parameter. Hooks and source
/// closures pass their argument both as `$in` and, when declared, as `$row`.
pub fn closure_arity(engine_state: &EngineState, closure: &Closure) -> usize {
    let block = engine_state.get_block(closure.block_id);
    block.signature.required_positional.len() + block.signature.optional_positional.len()
}

/// Run `closure` with `input` as `$in` (and as the first argument when the
/// closure declares one) and return its output.
pub fn call_closure(
    engine_state: &EngineState,
    stack: &Stack,
    closure: Closure,
    input: Value,
) -> Result<PipelineData, nu_protocol::ShellError> {
    let arity = closure_arity(engine_state, &closure);
    let mut eval = ClosureEvalOnce::new(engine_state, stack, closure);
    if arity > 0 {
        eval = eval.add_arg(input.clone())?;
    }
    eval.run_with_input(input.into_pipeline_data())
}

/// Run a hook closure with the state record and interpret its output.
pub fn run_hook(
    engine_state: &EngineState,
    stack: &Stack,
    closure: Closure,
    state: Value,
) -> HookOutcome {
    let span = state.span();
    match call_closure(engine_state, stack, closure, state).and_then(|data| data.into_value(span)) {
        Ok(value) => interpret(value),
        Err(err) => HookOutcome::Error(err.to_string()),
    }
}

/// Map a hook's return value onto an outcome.
pub fn interpret(value: Value) -> HookOutcome {
    match &value {
        Value::Nothing { .. } => HookOutcome::Nothing,
        Value::Record { val, .. } => {
            let action = val
                .get("action")
                .and_then(|v| v.as_str().ok())
                .map(str::to_ascii_lowercase);
            match action.as_deref() {
                Some("submit") => HookOutcome::Submit(
                    val.get("selected")
                        .cloned()
                        .unwrap_or_else(|| Value::nothing(Span::unknown())),
                ),
                Some("quit") => HookOutcome::Quit,
                _ => HookOutcome::Data(value),
            }
        }
        _ => HookOutcome::Data(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::Record;

    #[test]
    fn nothing_is_a_no_op() {
        assert!(matches!(
            interpret(Value::test_nothing()),
            HookOutcome::Nothing
        ));
    }

    #[test]
    fn action_records_end_the_tui() {
        let mut r = Record::new();
        r.insert("action", Value::test_string("submit"));
        r.insert("selected", Value::test_int(3));
        match interpret(Value::test_record(r)) {
            HookOutcome::Submit(v) => assert_eq!(v, Value::test_int(3)),
            other => panic!("expected submit, got {other:?}"),
        }
        let mut r = Record::new();
        r.insert("action", Value::test_string("quit"));
        assert!(matches!(
            interpret(Value::test_record(r)),
            HookOutcome::Quit
        ));
    }

    #[test]
    fn other_values_replace_data() {
        let list = Value::test_list(vec![Value::test_int(1)]);
        assert!(matches!(interpret(list), HookOutcome::Data(_)));
    }
}
