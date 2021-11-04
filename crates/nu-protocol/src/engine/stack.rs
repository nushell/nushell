use std::collections::HashMap;

use crate::{ShellError, Value, VarId};

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
    pub vars: HashMap<VarId, Value>,
    pub env_vars: HashMap<String, String>,
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
            env_vars: HashMap::new(),
        }
    }
    pub fn get_var(&self, var_id: VarId) -> Result<Value, ShellError> {
        if let Some(v) = self.vars.get(&var_id) {
            return Ok(v.clone());
        }
        Err(ShellError::InternalError("variable not found".into()))
    }

    pub fn add_var(&mut self, var_id: VarId, value: Value) {
        self.vars.insert(var_id, value);
    }

    pub fn add_env_var(&mut self, var: String, value: String) {
        self.env_vars.insert(var, value);
    }

    pub fn collect_captures(&self, captures: &[VarId]) -> Stack {
        let mut output = Stack::new();

        for capture in captures {
            // Note: this assumes we have calculated captures correctly and that commands
            // that take in a var decl will manually set this into scope when running the blocks
            if let Ok(value) = self.get_var(*capture) {
                output.vars.insert(*capture, value);
            }
        }

        // FIXME: this is probably slow
        output.env_vars = self.env_vars.clone();

        output
    }

    pub fn get_env_vars(&self) -> HashMap<String, String> {
        self.env_vars.clone()
    }

    pub fn get_env_var(&self, name: &str) -> Option<String> {
        if let Some(v) = self.env_vars.get(name) {
            return Some(v.to_string());
        }
        None
    }

    pub fn print_stack(&self) {
        println!("vars:");
        for (var, val) in &self.vars {
            println!("  {}: {:?}", var, val);
        }
        println!("env vars:");
        for (var, val) in &self.env_vars {
            println!("  {}: {:?}", var, val);
        }
    }
}
