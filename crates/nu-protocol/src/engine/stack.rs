use std::collections::HashMap;

use crate::{ShellError, Value, VarId};

#[derive(Debug, Clone)]
pub struct StackFrame {
    pub vars: HashMap<VarId, Value>,
    pub env_vars: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct Stack(Vec<StackFrame>);

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

impl Stack {
    pub fn new() -> Stack {
        Stack(vec![StackFrame {
            vars: HashMap::new(),
            env_vars: HashMap::new(),
        }])
    }
    pub fn get_var(&self, var_id: VarId) -> Result<Value, ShellError> {
        for frame in self.0.iter().rev() {
            if let Some(v) = frame.vars.get(&var_id) {
                return Ok(v.clone());
            }
        }
        Err(ShellError::InternalError("variable not found".into()))
    }

    pub fn add_var(&mut self, var_id: VarId, value: Value) {
        let frame = self
            .0
            .last_mut()
            .expect("internal error: can't access stack frame");
        frame.vars.insert(var_id, value);
    }

    pub fn add_env_var(&mut self, var: String, value: String) {
        let frame = self
            .0
            .last_mut()
            .expect("internal error: can't access stack frame");
        frame.env_vars.insert(var, value);
    }

    pub fn enter_scope(&self) -> Stack {
        // FIXME: VERY EXPENSIVE to clone entire stack
        let mut output = self.clone();
        output.0.push(StackFrame {
            vars: HashMap::new(),
            env_vars: HashMap::new(),
        });

        output
    }

    pub fn get_env_vars(&self) -> HashMap<String, String> {
        let mut output = HashMap::new();

        for frame in &self.0 {
            output.extend(frame.env_vars.clone().into_iter());
        }

        output
    }

    pub fn get_env_var(&self, name: &str) -> Option<String> {
        for frame in self.0.iter().rev() {
            if let Some(v) = frame.env_vars.get(name) {
                return Some(v.to_string());
            }
        }
        None
    }

    pub fn print_stack(&self) {
        for frame in self.0.iter().rev() {
            println!("===frame===");
            println!("vars:");
            for (var, val) in &frame.vars {
                println!("  {}: {:?}", var, val);
            }
            println!("env vars:");
            for (var, val) in &frame.env_vars {
                println!("  {}: {:?}", var, val);
            }
        }
    }
}
