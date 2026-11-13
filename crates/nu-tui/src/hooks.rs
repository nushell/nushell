//! The one closure contract shared by every hook: `tui bind`, menu actions,
//! `tui button`, `--on-select`, and the `tui run` refresh closure.
//!
//! A hook receives the current state record (the same shape `tui run`
//! returns) as `$in`, and as its first parameter when it declares one. What
//! it returns decides what happens:
//!
//! - `null` (or no output): nothing changes.
//! - a record with a string `action` field: `{action: submit, selected: ...}`
//!   ends the TUI with that value; `{action: quit}` quits; `context` writes
//!   the app's context record (`$state.context`); `focus`, `resize`,
//!   `scroll`, and `context` with an `id` change one widget (see
//!   [`HookAction`]). A list of such records applies each in turn. An
//!   unknown action, or a list mixing actions and other values, is an error.
//! - anything else: it replaces the shared data list, so every table, log,
//!   and tree that shows the outer pipeline's data redraws.

use nu_engine::ClosureEvalOnce;
use nu_protocol::engine::{Closure, EngineState, Stack};
use nu_protocol::{IntoPipelineData, PipelineData, ShellError, Span, Value};

#[derive(Debug, Clone)]
pub enum HookOutcome {
    Nothing,
    Data(Value),
    Submit(Value),
    Quit,
    Action(HookAction),
    /// A list of action records, applied in order.
    Many(Vec<HookOutcome>),
    Error(String),
}

/// A hook's request to change one widget, by id.
#[derive(Debug, Clone)]
pub enum HookAction {
    /// `{action: focus, id}`: move focus to the widget.
    Focus(String),
    /// `{action: resize, id, sizes}`: give a split new child sizes, in the
    /// forms `--sizes` takes.
    Resize(String, Vec<crate::widget::Size>),
    /// `{action: context, value}`: merge the record `value` into the app
    /// context key by key; with an `id`,
    /// into that widget's `views.<id>`. Closures that see the change run
    /// again.
    Context(Option<String>, Value),
    /// `{action: scroll, id, line}`: scroll a preview's text to `line`;
    /// a negative line counts from the end.
    Scroll(String, i64),
    /// `{action: columns, id, columns}`: show these columns of a table.
    Columns(String, Vec<String>),
    /// `{action: select, id, index}`: highlight a row of a list widget.
    Select(String, usize),
    /// `{action: suspend, hook, area?}`: give the terminal to `hook`, then
    /// treat what it returns as this hook's result. With `area`, the id of
    /// a widget, the TUI stays on screen round the hook's output, which
    /// starts at that widget's top-left corner and finds the rectangle in
    /// `$state.area`; without, the normal screen comes back meanwhile.
    Suspend { hook: Closure, area: Option<String> },
}

/// Whether the closure declares a positional parameter. Hooks and source
/// closures pass their argument both as `$in` and, when declared, as `$row`.
///
/// Fails when the closure's block is not available in this engine state,
/// which happens for a closure parsed after the running job was spawned.
/// The error is only ever formatted into preview text, so the label span
/// is left unknown.
pub fn closure_arity(engine_state: &EngineState, closure: &Closure) -> Result<usize, ShellError> {
    Ok(block_arity(
        engine_state.get_closure_block(closure, Span::unknown())?,
    ))
}

/// How many positional parameters the closure's block declares.
fn block_arity(block: &nu_protocol::ast::Block) -> usize {
    block.signature.required_positional.len() + block.signature.optional_positional.len()
}

/// Run `closure` with `input` as `$in` (and as the first argument when the
/// closure declares one) and return its output.
pub fn call_closure(
    engine_state: &EngineState,
    stack: &Stack,
    closure: Closure,
    input: Value,
) -> Result<PipelineData, ShellError> {
    let span = input.span();
    let mut eval = ClosureEvalOnce::try_new(engine_state, stack, closure, span)?;
    let arity = block_arity(eval.block());
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
            let id = || {
                val.get("id")
                    .and_then(|v| v.as_str().ok())
                    .map(str::to_string)
                    .ok_or_else(|| {
                        format!("{} action needs an `id`", action.as_deref().unwrap_or(""))
                    })
            };
            let outcome = match action.as_deref() {
                Some("submit") => Ok(HookOutcome::Submit(
                    val.get("selected")
                        .cloned()
                        .unwrap_or_else(|| Value::nothing(Span::unknown())),
                )),
                Some("quit") => Ok(HookOutcome::Quit),
                Some("focus") => id().map(|id| HookOutcome::Action(HookAction::Focus(id))),
                Some("resize") => id().and_then(|id| {
                    let sizes = match val.get("sizes") {
                        Some(Value::List { vals, .. }) => vals
                            .iter()
                            .map(crate::widgets::split::parse_size)
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|e| e.to_string())?,
                        _ => return Err("resize action needs a `sizes` list".to_string()),
                    };
                    Ok(HookOutcome::Action(HookAction::Resize(id, sizes)))
                }),
                Some("context") => {
                    let id = val.get("id").map(|id| id.as_str().map(str::to_string));
                    match (id, val.get("value")) {
                        (Some(Err(_)), _) => {
                            Err("context action's `id` must be a string".to_string())
                        }
                        (id, Some(value @ Value::Record { .. })) => Ok(HookOutcome::Action(
                            HookAction::Context(id.and_then(Result::ok), value.clone()),
                        )),
                        _ => Err("context action needs a record `value`".to_string()),
                    }
                }
                Some("scroll") => id().and_then(|id| match val.get("line").map(Value::as_int) {
                    Some(Ok(line)) => Ok(HookOutcome::Action(HookAction::Scroll(id, line))),
                    _ => Err("scroll action needs an integer `line`".to_string()),
                }),
                Some("suspend") => match val.get("hook") {
                    Some(Value::Closure { val: hook, .. }) => {
                        let area = val
                            .get("area")
                            .and_then(|v| v.as_str().ok())
                            .map(str::to_string);
                        Ok(HookOutcome::Action(HookAction::Suspend {
                            hook: *hook.clone(),
                            area,
                        }))
                    }
                    _ => Err("suspend action needs a `hook` closure".to_string()),
                },
                // `{action: data, value}` is a returned value that can sit in
                // a list of actions, so rows and a selection go together.
                Some("data") => val
                    .get("value")
                    .cloned()
                    .map(HookOutcome::Data)
                    .ok_or_else(|| "data action needs a `value`".to_string()),
                Some("select") => id().and_then(|id| match val.get("index").map(Value::as_int) {
                    Some(Ok(index)) if index >= 0 => {
                        Ok(HookOutcome::Action(HookAction::Select(id, index as usize)))
                    }
                    _ => Err("select action needs an `index` of 0 or more".to_string()),
                }),
                Some("columns") => id().and_then(|id| match val.get("columns") {
                    Some(Value::List { vals, .. }) => vals
                        .iter()
                        .map(|v| v.as_str().map(str::to_string))
                        .collect::<Result<Vec<_>, _>>()
                        .map(|columns| HookOutcome::Action(HookAction::Columns(id, columns)))
                        .map_err(|_| "columns action needs a list of column names".to_string()),
                    _ => Err("columns action needs a `columns` list".to_string()),
                }),
                // A string `action` is reserved, so a typo is an error
                // rather than rows that wipe out the data.
                Some(other) => Err(format!("unknown action `{other}`; see `help tui bind`")),
                None => Ok(HookOutcome::Data(value)),
            };
            outcome.unwrap_or_else(HookOutcome::Error)
        }
        Value::List { vals, .. } if vals.iter().any(is_action) => {
            if vals.iter().all(is_action) {
                HookOutcome::Many(vals.iter().cloned().map(interpret).collect())
            } else {
                HookOutcome::Error(
                    "a list of actions cannot hold other values; see `help tui bind`".to_string(),
                )
            }
        }
        _ => HookOutcome::Data(value),
    }
}

/// A record with a string `action` field.
fn is_action(value: &Value) -> bool {
    value
        .as_record()
        .ok()
        .and_then(|r| r.get("action"))
        .is_some_and(|a| a.as_str().is_ok())
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

    fn action(fields: &[(&str, Value)]) -> Value {
        let mut r = Record::new();
        for (k, v) in fields {
            r.insert(*k, v.clone());
        }
        Value::test_record(r)
    }

    #[test]
    fn widget_actions_name_their_widget() {
        let focus = action(&[
            ("action", Value::test_string("focus")),
            ("id", Value::test_string("view")),
        ]);
        assert!(matches!(
            interpret(focus),
            HookOutcome::Action(HookAction::Focus(id)) if id == "view"
        ));
        let scroll = action(&[
            ("action", Value::test_string("scroll")),
            ("id", Value::test_string("view")),
            ("line", Value::test_int(-1)),
        ]);
        assert!(matches!(
            interpret(scroll),
            HookOutcome::Action(HookAction::Scroll(id, -1)) if id == "view"
        ));
        let resize = action(&[
            ("action", Value::test_string("resize")),
            ("id", Value::test_string("layout")),
            (
                "sizes",
                Value::test_list(vec![Value::test_int(0), Value::test_string("1fr")]),
            ),
        ]);
        assert!(matches!(
            interpret(resize),
            HookOutcome::Action(HookAction::Resize(id, sizes))
                if id == "layout" && sizes == vec![crate::widget::Size::Length(0), crate::widget::Size::Fill(1)]
        ));
        let columns = action(&[
            ("action", Value::test_string("columns")),
            ("id", Value::test_string("t")),
            (
                "columns",
                Value::test_list(vec![Value::test_string("a"), Value::test_string("b")]),
            ),
        ]);
        assert!(matches!(
            interpret(columns),
            HookOutcome::Action(HookAction::Columns(id, cols)) if id == "t" && cols == ["a", "b"]
        ));
        let hook = Closure {
            block_id: nu_protocol::BlockId::new(0),
            captures: Vec::new(),
        };
        let suspend = action(&[
            ("action", Value::test_string("suspend")),
            ("hook", Value::test_closure(hook)),
            ("area", Value::test_string("view")),
        ]);
        assert!(matches!(
            interpret(suspend),
            HookOutcome::Action(HookAction::Suspend { area: Some(area), .. }) if area == "view"
        ));
        let select = action(&[
            ("action", Value::test_string("select")),
            ("id", Value::test_string("t")),
            ("index", Value::test_int(2)),
        ]);
        assert!(matches!(
            interpret(select),
            HookOutcome::Action(HookAction::Select(id, 2)) if id == "t"
        ));
        let negative = action(&[
            ("action", Value::test_string("select")),
            ("id", Value::test_string("t")),
            ("index", Value::test_int(-1)),
        ]);
        assert!(matches!(interpret(negative), HookOutcome::Error(_)));
        let data = action(&[
            ("action", Value::test_string("data")),
            ("value", Value::test_list(vec![Value::test_int(1)])),
        ]);
        assert!(matches!(
            interpret(data),
            HookOutcome::Data(Value::List { .. })
        ));
        let no_hook = action(&[("action", Value::test_string("suspend"))]);
        assert!(matches!(interpret(no_hook), HookOutcome::Error(_)));
        // Missing pieces are errors, not data.
        let no_id = action(&[("action", Value::test_string("focus"))]);
        assert!(matches!(interpret(no_id), HookOutcome::Error(_)));
        let bad_line = action(&[
            ("action", Value::test_string("scroll")),
            ("id", Value::test_string("v")),
        ]);
        assert!(matches!(interpret(bad_line), HookOutcome::Error(_)));
    }

    #[test]
    fn a_list_of_action_records_is_applied_in_order() {
        let list = Value::test_list(vec![
            action(&[
                ("action", Value::test_string("focus")),
                ("id", Value::test_string("a")),
            ]),
            action(&[("action", Value::test_string("quit"))]),
        ]);
        match interpret(list) {
            HookOutcome::Many(items) => {
                assert_eq!(items.len(), 2);
                assert!(matches!(items[1], HookOutcome::Quit));
            }
            other => panic!("expected many, got {other:?}"),
        }
        // Plain rows stay data even when a column is called `action`.
        let rows = Value::test_list(vec![action(&[("action", Value::test_int(1))])]);
        assert!(matches!(interpret(rows), HookOutcome::Data(_)));
        // A list mixing actions with anything else is an error, not data.
        let mixed = Value::test_list(vec![
            action(&[("action", Value::test_string("quit"))]),
            Value::test_int(5),
        ]);
        assert!(matches!(interpret(mixed), HookOutcome::Error(_)));
    }

    #[test]
    fn malformed_actions_are_errors_not_data() {
        let unknown = action(&[("action", Value::test_string("bogus"))]);
        assert!(
            matches!(interpret(unknown), HookOutcome::Error(e) if e.starts_with("unknown action `bogus`"))
        );
        let not_a_record = action(&[
            ("action", Value::test_string("context")),
            ("value", Value::test_int(5)),
        ]);
        assert!(matches!(interpret(not_a_record), HookOutcome::Error(_)));
        let no_value = action(&[("action", Value::test_string("context"))]);
        assert!(matches!(interpret(no_value), HookOutcome::Error(_)));
        let no_rows = action(&[("action", Value::test_string("data"))]);
        assert!(matches!(interpret(no_rows), HookOutcome::Error(_)));
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
