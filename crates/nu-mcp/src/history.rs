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
/// When the buffer is full, oldest entries are evicted to make room for new ones.
#[derive(Clone)]
pub struct History {
    buffer: VecDeque<Value>,
    var_id: VarId,
}

impl History {
    /// Creates a new history buffer and registers the `$history` variable.
    pub fn new(engine_state: &mut EngineState) -> Self {
        let var_id = register_history_variable(engine_state);
        Self {
            buffer: VecDeque::new(),
            var_id,
        }
    }

    /// Returns the variable ID for `$history`.
    pub fn var_id(&self) -> VarId {
        self.var_id
    }

    /// Pushes a value to the history, evicting the oldest entry if at capacity.
    ///
    /// Returns the index at which the value was stored.
    pub fn push(&mut self, value: Value, engine_state: &EngineState, stack: &Stack) -> usize {
        let limit = history_limit(engine_state, stack);
        if self.buffer.len() >= limit {
            self.buffer.pop_front();
        }
        let index = self.buffer.len();
        self.buffer.push_back(value);
        index
    }

    /// Creates a `Value::list` containing all history entries for use in evaluation.
    pub fn as_value(&self) -> Value {
        let list: Vec<Value> = self.buffer.iter().cloned().collect();
        Value::list(list, Span::unknown())
    }
}

fn register_history_variable(engine_state: &mut EngineState) -> VarId {
    let mut working_set = StateWorkingSet::new(engine_state);
    let var_id = working_set.add_variable(
        b"history".to_vec(),
        Span::unknown(),
        Type::List(Box::new(Type::Any)),
        false,
    );
    let delta = working_set.render();
    engine_state
        .merge_delta(delta)
        .expect("failed to register $history variable");
    var_id
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
