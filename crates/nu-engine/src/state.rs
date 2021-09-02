use nu_parser::ParserState;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use nu_protocol::{ShellError, Value, VarId};

pub struct State {
    pub parser_state: Rc<RefCell<ParserState>>,
    pub stack: Stack,
}

impl State {
    pub fn get_var(&self, var_id: VarId) -> Result<Value, ShellError> {
        self.stack.get_var(var_id)
    }

    pub fn enter_scope(&self) -> State {
        Self {
            parser_state: self.parser_state.clone(),
            stack: self.stack.clone().enter_scope(),
        }
    }

    pub fn add_var(&self, var_id: VarId, value: Value) {
        self.stack.add_var(var_id, value);
    }

    pub fn add_env_var(&self, var: String, value: String) {
        self.stack.add_env_var(var, value);
    }

    pub fn print_stack(&self) {
        self.stack.print_stack();
    }
}

#[derive(Debug)]
pub struct StackFrame {
    pub vars: HashMap<VarId, Value>,
    pub env_vars: HashMap<String, String>,
    pub parent: Option<Stack>,
}

#[derive(Clone, Debug)]
pub struct Stack(Rc<RefCell<StackFrame>>);

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

impl Stack {
    pub fn new() -> Stack {
        Stack(Rc::new(RefCell::new(StackFrame {
            vars: HashMap::new(),
            env_vars: HashMap::new(),
            parent: None,
        })))
    }
    pub fn get_var(&self, var_id: VarId) -> Result<Value, ShellError> {
        let this = self.0.borrow();
        match this.vars.get(&var_id) {
            Some(v) => Ok(v.clone()),
            _ => {
                if let Some(parent) = &this.parent {
                    parent.get_var(var_id)
                } else {
                    Err(ShellError::InternalError("variable not found".into()))
                }
            }
        }
    }

    pub fn add_var(&self, var_id: VarId, value: Value) {
        let mut this = self.0.borrow_mut();
        this.vars.insert(var_id, value);
    }

    pub fn add_env_var(&self, var: String, value: String) {
        let mut this = self.0.borrow_mut();
        this.env_vars.insert(var, value);
    }

    pub fn enter_scope(self) -> Stack {
        Stack(Rc::new(RefCell::new(StackFrame {
            vars: HashMap::new(),
            env_vars: HashMap::new(),
            parent: Some(self),
        })))
    }

    pub fn print_stack(&self) {
        println!("===frame===");
        println!("vars:");
        for (var, val) in &self.0.borrow().vars {
            println!("  {}: {:?}", var, val);
        }
        println!("env vars:");
        for (var, val) in &self.0.borrow().env_vars {
            println!("  {}: {:?}", var, val);
        }
        if let Some(parent) = &self.0.borrow().parent {
            parent.print_stack()
        }
    }
}
