use nu_protocol::{
    Span, Type, Value, VarId,
    engine::{EngineState, Stack, StateWorkingSet},
};
use std::collections::VecDeque;

const HISTORY_LIMIT_ENV_VAR: &str = "NU_MCP_HISTORY_LIMIT";
const DEFAULT_HISTORY_LIMIT: usize = 100;

/// Ring buffer for storing command output history.
///
/// Maintains a fixed-size buffer of values that can be accessed via the `$history` variable.
/// When the buffer is full, oldest entries are evicted to make room for new ones. Indices never
/// shift: an entry keeps the index it was stored at, and an evicted index reads as `null`.
#[derive(Clone)]
pub struct History {
    buffer: VecDeque<Value>,
    /// Number of entries evicted so far, which is also the index of the oldest held entry.
    evicted: usize,
    var_id: Option<VarId>,
}

impl History {
    /// Creates a new history buffer and registers the `$history` variable.
    pub fn new(engine_state: &mut EngineState) -> Self {
        let var_id = register_history_variable(engine_state);
        Self {
            buffer: VecDeque::new(),
            evicted: 0,
            var_id,
        }
    }

    /// Returns the variable ID for `$history`.
    pub fn var_id(&self) -> Option<VarId> {
        self.var_id
    }

    /// Pushes a value to the history, evicting the oldest entries until it fits the limit.
    ///
    /// The new value is always kept, even with a limit of 0, because the response that
    /// reports a truncated output points at it. Returns the index at which the value was
    /// stored; indices keep increasing across evictions.
    pub fn push(&mut self, value: Value, engine_state: &EngineState, stack: &Stack) -> usize {
        let limit = history_limit(engine_state, stack).max(1);
        while self.buffer.len() >= limit {
            self.buffer.pop_front();
            self.evicted += 1;
        }
        self.buffer.push_back(value);
        self.evicted + self.buffer.len() - 1
    }

    /// Creates a `Value::list` containing all history entries for use in evaluation.
    ///
    /// Evicted entries are `null` placeholders so that `$history.<index>` keeps pointing at
    /// the entry stored at that index.
    pub fn as_value(&self) -> Value {
        // No source span available -- history is a runtime-constructed value
        let list: Vec<Value> = std::iter::repeat_n(Value::nothing(Span::unknown()), self.evicted)
            .chain(self.buffer.iter().cloned())
            .collect();
        Value::list(list, Span::unknown())
    }
}

fn register_history_variable(engine_state: &mut EngineState) -> Option<VarId> {
    let mut working_set = StateWorkingSet::new(engine_state);

    if let Some(existing_var_id) = working_set.find_variable(b"history") {
        return Some(existing_var_id);
    }

    // No source span -- $history is a synthetic variable, not from parsed source
    let var_id = working_set.add_variable(
        b"history".to_vec(),
        Span::unknown(),
        Type::List(Box::new(Type::Any)),
        false,
    );
    let delta = working_set.render();
    match engine_state.merge_delta(delta) {
        Ok(()) => Some(var_id),
        Err(err) => {
            tracing::warn!("failed to register $history variable: {err}");
            None
        }
    }
}

/// Returns the history limit (max number of entries in the ring buffer).
///
/// Defaults to 100 entries. Can be overridden via `NU_MCP_HISTORY_LIMIT` env var.
fn history_limit(engine_state: &EngineState, stack: &Stack) -> usize {
    stack
        .get_env_var(engine_state, HISTORY_LIMIT_ENV_VAR)
        .and_then(|v| v.as_int().ok())
        .and_then(|i| usize::try_from(i).ok())
        .unwrap_or(DEFAULT_HISTORY_LIMIT)
}
