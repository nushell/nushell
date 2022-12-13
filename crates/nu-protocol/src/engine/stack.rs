use std::collections::{HashMap, HashSet};

use crate::engine::EngineState;
use crate::engine::DEFAULT_OVERLAY_NAME;
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

impl Stack {
    pub fn new() -> Stack {
        Stack {
            vars: HashMap::new(),
            env_vars: vec![],
            env_hidden: HashMap::new(),
            active_overlays: vec![DEFAULT_OVERLAY_NAME.to_string()],
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
                self.env_vars.push(HashMap::from([(
                    last_overlay.into(),
                    HashMap::from([(var, value)]),
                )]));
            }
        } else {
            // TODO: Remove panic
            panic!("internal error: no active overlay");
        }
    }

    pub fn last_overlay_name(&self) -> Result<String, ShellError> {
        self.active_overlays
            .last()
            .cloned()
            .ok_or_else(|| ShellError::NushellFailed("No active overlay".into()))
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

    /// Get flattened environment variables only from the stack and one overlay
    pub fn get_stack_overlay_env_vars(&self, overlay_name: &str) -> HashMap<String, Value> {
        let mut result = HashMap::new();

        for scope in &self.env_vars {
            if let Some(active_overlay) = self.active_overlays.iter().find(|n| n == &overlay_name) {
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
                env_hidden.contains(name)
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
                env_hidden.contains(name)
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

    pub fn has_env_overlay(&self, name: &str, engine_state: &EngineState) -> bool {
        for scope in self.env_vars.iter().rev() {
            if scope.contains_key(name) {
                return true;
            }
        }

        engine_state.env_vars.contains_key(name)
    }

    pub fn is_overlay_active(&self, name: &String) -> bool {
        self.active_overlays.contains(name)
    }

    pub fn add_overlay(&mut self, name: String) {
        self.active_overlays.retain(|o| o != &name);
        self.active_overlays.push(name);
    }

    pub fn remove_overlay(&mut self, name: &String) {
        self.active_overlays.retain(|o| o != name);
    }
}

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}
