use std::collections::{HashMap, HashSet};

use crate::engine::EngineState;
use crate::{ShellError, Span, Value, VarId};

/// Environment variables per overlay
pub type EnvVars = HashMap<String, HashMap<String, Value>>;

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
    pub env_vars: Vec<EnvVars>,
    /// Tells which environment variables from engine state are hidden, per overlay.
    pub env_hidden: HashMap<String, HashSet<String>>,
    /// List of active overlays
    pub active_overlays: Vec<String>,
}

// impl Default for Stack {
//     fn default() -> Self {
//         Self::new()
//     }
// }

impl Stack {
    pub fn new(engine_state: &EngineState) -> Stack {
        Stack {
            vars: HashMap::new(),
            env_vars: vec![],
            env_hidden: HashMap::new(),
            active_overlays: engine_state
                .active_overlay_names()
                .iter()
                .map(|name_bytes| String::from_utf8_lossy(name_bytes).to_string())
                .collect(),
        }
    }

    pub fn with_env(
        &mut self,
        env_vars: &[EnvVars],
        env_hidden: &HashMap<String, HashSet<String>>,
    ) {
        // Do not clone the environment if it hasn't changed
        if self.env_vars.iter().any(|scope| !scope.is_empty()) {
            self.env_vars = env_vars.to_owned();
        }

        if !self.env_hidden.is_empty() {
            self.env_hidden = env_hidden.to_owned();
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
        if let Some(last_overlay) = self.active_overlays.last() {
            if let Some(env_hidden) = self.env_hidden.get_mut(last_overlay) {
                // if the env var was hidden, let's activate it again
                env_hidden.remove(&var);
            }

            if let Some(scope) = self.env_vars.last_mut() {
                if let Some(env_vars) = scope.get_mut(last_overlay) {
                    env_vars.insert(var, value);
                } else {
                    scope.insert(last_overlay.into(), HashMap::from([(var, value)]));
                }
            } else {
                // self.env_vars.push(HashMap::from([(var, value)]));
                self.env_vars.push(HashMap::from([(
                    last_overlay.into(),
                    HashMap::from([(var, value)]),
                )]));
            }
        } else {
            panic!("internal error: no active overlay");
        }
    }

    pub fn captures_to_stack(&self, captures: &HashMap<VarId, Value>) -> Stack {
        // FIXME: this is probably slow
        let mut env_vars = self.env_vars.clone();
        env_vars.push(HashMap::new());

        Stack {
            vars: captures.clone(),
            env_vars,
            env_hidden: HashMap::new(),
            active_overlays: self.active_overlays.clone(),
        }
    }

    pub fn gather_captures(&self, captures: &[VarId]) -> Stack {
        let mut vars = HashMap::new();

        let fake_span = Span::new(0, 0);

        for capture in captures {
            // Note: this assumes we have calculated captures correctly and that commands
            // that take in a var decl will manually set this into scope when running the blocks
            if let Ok(value) = self.get_var(*capture, fake_span) {
                vars.insert(*capture, value);
            }
        }

        let mut env_vars = self.env_vars.clone();
        env_vars.push(HashMap::new());

        Stack {
            vars,
            env_vars,
            env_hidden: HashMap::new(),
            active_overlays: self.active_overlays.clone(),
        }
    }

    /// Flatten the env var scope frames into one frame
    pub fn get_env_vars(&self, engine_state: &EngineState) -> HashMap<String, Value> {
        let mut result = HashMap::new();

        for active_overlay in self.active_overlays.iter() {
            if let Some(env_vars) = engine_state.env_vars.get(active_overlay) {
                result.extend(
                    env_vars
                        .iter()
                        .filter(|(k, _)| {
                            if let Some(env_hidden) = self.env_hidden.get(active_overlay) {
                                !env_hidden.contains(*k)
                            } else {
                                // nothing has been hidden in this overlay
                                true
                            }
                        })
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<HashMap<String, Value>>(),
                );
            }
        }

        result.extend(self.get_stack_env_vars());

        result
    }

    /// Get flattened environment variables only from the stack
    pub fn get_stack_env_vars(&self) -> HashMap<String, Value> {
        let mut result = HashMap::new();

        for scope in &self.env_vars {
            for active_overlay in self.active_overlays.iter() {
                if let Some(env_vars) = scope.get(active_overlay) {
                    result.extend(env_vars.clone());
                }
            }
        }

        result
    }

    /// Same as get_env_vars, but returns only the names as a HashSet
    pub fn get_env_var_names(&self, engine_state: &EngineState) -> HashSet<String> {
        let mut result = HashSet::new();

        for active_overlay in self.active_overlays.iter() {
            if let Some(env_vars) = engine_state.env_vars.get(active_overlay) {
                result.extend(
                    env_vars
                        .keys()
                        .filter(|k| {
                            if let Some(env_hidden) = self.env_hidden.get(active_overlay) {
                                !env_hidden.contains(*k)
                            } else {
                                // nothing has been hidden in this overlay
                                true
                            }
                        })
                        .cloned()
                        .collect::<HashSet<String>>(),
                );
            }
        }

        for scope in &self.env_vars {
            for active_overlay in self.active_overlays.iter() {
                if let Some(env_vars) = scope.get(active_overlay) {
                    result.extend(env_vars.keys().cloned().collect::<HashSet<String>>());
                }
            }
        }

        result
    }

    pub fn get_env_var(&self, engine_state: &EngineState, name: &str) -> Option<Value> {
        for scope in self.env_vars.iter().rev() {
            for active_overlay in self.active_overlays.iter().rev() {
                if let Some(env_vars) = scope.get(active_overlay) {
                    if let Some(v) = env_vars.get(name) {
                        return Some(v.clone());
                    }
                }
            }
        }

        for active_overlay in self.active_overlays.iter().rev() {
            let is_hidden = if let Some(env_hidden) = self.env_hidden.get(active_overlay) {
                !env_hidden.contains(name)
            } else {
                false
            };

            if !is_hidden {
                if let Some(env_vars) = engine_state.env_vars.get(active_overlay) {
                    if let Some(v) = env_vars.get(name) {
                        return Some(v.clone());
                    }
                }
            }
        }

        None
    }

    pub fn has_env_var(&self, engine_state: &EngineState, name: &str) -> bool {
        for scope in self.env_vars.iter().rev() {
            for active_overlay in self.active_overlays.iter().rev() {
                if let Some(env_vars) = scope.get(active_overlay) {
                    if env_vars.contains_key(name) {
                        return true;
                    }
                }
            }
        }

        for active_overlay in self.active_overlays.iter().rev() {
            let is_hidden = if let Some(env_hidden) = self.env_hidden.get(active_overlay) {
                !env_hidden.contains(name)
            } else {
                false
            };

            if !is_hidden {
                if let Some(env_vars) = engine_state.env_vars.get(active_overlay) {
                    if env_vars.contains_key(name) {
                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn remove_env_var(&mut self, engine_state: &EngineState, name: &str) -> Option<Value> {
        for scope in self.env_vars.iter_mut().rev() {
            for active_overlay in self.active_overlays.iter().rev() {
                if let Some(env_vars) = scope.get_mut(active_overlay) {
                    if let Some(v) = env_vars.remove(name) {
                        return Some(v);
                    }
                }
            }
        }

        for active_overlay in self.active_overlays.iter().rev() {
            if let Some(env_vars) = engine_state.env_vars.get(active_overlay) {
                if let Some(val) = env_vars.get(name) {
                    if let Some(env_hidden) = self.env_hidden.get_mut(active_overlay) {
                        env_hidden.insert(name.into());
                    } else {
                        self.env_hidden
                            .insert(active_overlay.into(), HashSet::from([name.into()]));
                    }

                    return Some(val.clone());
                }
            }
        }

        None
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

    // pub fn print_stack(&self) {
    //     println!("vars:");
    //     for (var, val) in &self.vars {
    //         println!("  {}: {:?}", var, val);
    //     }
    //     for (i, scope) in self.env_vars.iter().rev().enumerate() {
    //         println!("env vars, scope {} (from the last);", i);
    //         for (var, val) in scope {
    //             println!("  {}: {:?}", var, val.clone().debug_value());
    //         }
    //     }
    // }
}
