use std::collections::{HashMap, HashSet};

use crate::engine::EngineState;
use crate::{ShellError, Span, Value, VarId};

/// A runtime value stack used during evaluation
///
/// A note on implementation:
///
/// We previously set up the stack in a traditional way, where stack frames had parents which would
/// represent other frames that you might return to when exiting a function.
///
/// While experimenting with blocks, we found that we needed to have closure captures of variables
/// seen outside of the blocks, so that they blocks could be run in a way that was both thread-safe
/// and followed the restrictions for closures applied to iterators. The end result left us with
/// closure-captured single stack frames that blocks could see.
///
/// Blocks make up the only scope and stack definition abstraction in Nushell. As a result, we were
/// creating closure captures at any point we wanted to have a Block value we could safely evaluate
/// in any context. This meant that the parents were going largely unused, with captured variables
/// taking their place. The end result is this, where we no longer have separate frames, but instead
/// use the Stack as a way of representing the local and closure-captured state.
#[derive(Debug, Clone)]
pub struct Stack {
    /// Variables
    pub vars: HashMap<VarId, Value>,
    /// Environment variables arranged as a stack to be able to recover values from parent scopes
    pub env_vars: Vec<HashMap<String, Value>>,
    /// Tells which environment variables from engine state are hidden. We don't need to track the
    /// env vars in the stack since we can just delete them.
    pub env_hidden: HashSet<String>,
}

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

impl Stack {
    pub fn new() -> Stack {
        Stack {
            vars: HashMap::new(),
            env_vars: vec![],
            env_hidden: HashSet::new(),
        }
    }

    pub fn with_env(&mut self, env_vars: &[HashMap<String, Value>], env_hidden: &HashSet<String>) {
        // Do not clone the environment if it hasn't changed
        if self.env_vars.iter().any(|scope| !scope.is_empty()) {
            self.env_vars = env_vars.to_owned();
        }

        if !self.env_hidden.is_empty() {
            self.env_hidden = env_hidden.clone();
        }
    }

    pub fn get_var(&self, var_id: VarId, span: Span) -> Result<Value, ShellError> {
        if let Some(v) = self.vars.get(&var_id) {
            return Ok(v.clone().with_span(span));
        }

        Err(ShellError::VariableNotFoundAtRuntime(span))
    }

    pub fn get_var_with_origin(&self, var_id: VarId, span: Span) -> Result<Value, ShellError> {
        if let Some(v) = self.vars.get(&var_id) {
            return Ok(v.clone());
        }

        Err(ShellError::VariableNotFoundAtRuntime(span))
    }

    pub fn add_var(&mut self, var_id: VarId, value: Value) {
        self.vars.insert(var_id, value);
    }

    pub fn add_env_var(&mut self, var: String, value: Value) {
        // if the env var was hidden, let's activate it again
        self.env_hidden.remove(&var);

        if let Some(scope) = self.env_vars.last_mut() {
            scope.insert(var, value);
        } else {
            self.env_vars.push(HashMap::from([(var, value)]));
        }
    }

    pub fn captures_to_stack(&self, captures: &HashMap<VarId, Value>) -> Stack {
        let mut output = Stack::new();

        output.vars = captures.clone();

        // FIXME: this is probably slow
        output.env_vars = self.env_vars.clone();
        output.env_vars.push(HashMap::new());

        output
    }

    pub fn gather_captures(&self, captures: &[VarId]) -> Stack {
        let mut output = Stack::new();

        let fake_span = Span::new(0, 0);

        for capture in captures {
            // Note: this assumes we have calculated captures correctly and that commands
            // that take in a var decl will manually set this into scope when running the blocks
            if let Ok(value) = self.get_var(*capture, fake_span) {
                output.vars.insert(*capture, value);
            }
        }

        // FIXME: this is probably slow
        output.env_vars = self.env_vars.clone();
        output.env_vars.push(HashMap::new());

        output
    }

    /// Flatten the env var scope frames into one frame
    pub fn get_env_vars(&self, engine_state: &EngineState) -> HashMap<String, Value> {
        // TODO: We're collecting im::HashMap to HashMap here. It might make sense to make these
        // the same data structure.
        let mut result: HashMap<String, Value> = engine_state
            .env_vars
            .iter()
            .filter(|(k, _)| !self.env_hidden.contains(*k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        for scope in &self.env_vars {
            result.extend(scope.clone());
        }

        result
    }

    /// Same as get_env_vars, but returns only the names as a HashSet
    pub fn get_env_var_names(&self, engine_state: &EngineState) -> HashSet<String> {
        let mut result: HashSet<String> = engine_state
            .env_vars
            .keys()
            .filter(|k| !self.env_hidden.contains(*k))
            .cloned()
            .collect();

        for scope in &self.env_vars {
            let scope_keys: HashSet<String> = scope.keys().cloned().collect();
            result.extend(scope_keys);
        }

        result
    }

    pub fn get_env_var(&self, engine_state: &EngineState, name: &str) -> Option<Value> {
        for scope in self.env_vars.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Some(v.clone());
            }
        }

        if self.env_hidden.contains(name) {
            None
        } else {
            engine_state.env_vars.get(name).cloned()
        }
    }

    pub fn has_env_var(&self, engine_state: &EngineState, name: &str) -> bool {
        for scope in self.env_vars.iter().rev() {
            if scope.contains_key(name) {
                return true;
            }
        }

        if self.env_hidden.contains(name) {
            false
        } else {
            engine_state.env_vars.contains_key(name)
        }
    }

    pub fn remove_env_var(&mut self, engine_state: &EngineState, name: &str) -> Option<Value> {
        for scope in self.env_vars.iter_mut().rev() {
            if let Some(v) = scope.remove(name) {
                return Some(v);
            }
        }

        if self.env_hidden.contains(name) {
            // the environment variable is already hidden
            None
        } else if let Some(val) = engine_state.env_vars.get(name) {
            // the environment variable was found in the engine state => mark it as hidden
            self.env_hidden.insert(name.to_string());
            Some(val.clone())
        } else {
            None
        }
    }

    // pub fn get_config(&self) -> Result<Config, ShellError> {
    //     let config = self.get_var(CONFIG_VARIABLE_ID, Span::new(0, 0));

    //     match config {
    //         Ok(config) => config.into_config(),
    //         Err(e) => Err(e),
    //     }
    // }

    // pub fn update_config(&mut self, name: &str, value: Value) {
    //     if let Some(Value::Record { cols, vals, .. }) = self.vars.get_mut(&CONFIG_VARIABLE_ID) {
    //         for col_val in cols.iter().zip(vals.iter_mut()) {
    //             if col_val.0 == name {
    //                 *col_val.1 = value;
    //                 return;
    //             }
    //         }
    //         cols.push(name.to_string());
    //         vals.push(value);
    //     }
    // }

    pub fn print_stack(&self) {
        println!("vars:");
        for (var, val) in &self.vars {
            println!("  {}: {:?}", var, val);
        }
        for (i, scope) in self.env_vars.iter().rev().enumerate() {
            println!("env vars, scope {} (from the last);", i);
            for (var, val) in scope {
                println!("  {}: {:?}", var, val.clone().debug_value());
            }
        }
    }
}
