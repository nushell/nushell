use std::collections::HashMap;

use crate::{ShellError, Value, VarId};

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
            output.vars.insert(
                *capture,
                self.get_var(*capture)
                    .expect("internal error: capture of missing variable"),
            );
        }

        output
    }

    // pub fn enter_scope(&self) -> Stack {
    //     // FIXME: VERY EXPENSIVE to clone entire stack
    //     let mut output = self.clone();
    //     output.0.push(StackFrame {
    //         vars: HashMap::new(),
    //         env_vars: HashMap::new(),
    //     });

    //     output
    // }

    pub fn get_env_vars(&self) -> HashMap<String, String> {
        // let mut output = HashMap::new();

        // for frame in &self.0 {
        //     output.extend(frame.env_vars.clone().into_iter());
        // }

        // output
        self.env_vars.clone()
    }

    pub fn get_env_var(&self, name: &str) -> Option<String> {
        // for frame in self.0.iter().rev() {
        if let Some(v) = self.env_vars.get(name) {
            return Some(v.to_string());
        }
        // }
        None
    }

    pub fn print_stack(&self) {
        // for frame in self.0.iter().rev() {
        // println!("===frame===");
        println!("vars:");
        for (var, val) in &self.vars {
            println!("  {}: {:?}", var, val);
        }
        println!("env vars:");
        for (var, val) in &self.env_vars {
            println!("  {}: {:?}", var, val);
        }
        // }
    }
}
