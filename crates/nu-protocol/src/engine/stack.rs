use std::{
    collections::{HashMap, HashSet},
    mem,
    ops::{Deref, DerefMut},
};

use crate::{
    engine::{EngineState, DEFAULT_OVERLAY_NAME},
    IoStream, ShellError, Span, Value, VarId, ENV_VARIABLE_ID, NU_VARIABLE_ID,
};

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
    pub vars: Vec<(VarId, Value)>,
    /// Environment variables arranged as a stack to be able to recover values from parent scopes
    pub env_vars: Vec<EnvVars>,
    /// Tells which environment variables from engine state are hidden, per overlay.
    pub env_hidden: HashMap<String, HashSet<String>>,
    /// List of active overlays
    pub active_overlays: Vec<String>,
    pub recursion_count: u64,
    stdout: IoStream,
    stderr: IoStream,
    parent_stdout: Option<IoStream>,
    parent_stderr: Option<IoStream>,
}

impl Stack {
    pub fn new(stdout: IoStream, stderr: IoStream) -> Stack {
        Stack {
            vars: vec![],
            env_vars: vec![],
            env_hidden: HashMap::new(),
            active_overlays: vec![DEFAULT_OVERLAY_NAME.to_string()],
            recursion_count: 0,
            stdout,
            stderr,
            parent_stdout: None,
            parent_stderr: None,
        }
    }

    pub fn with_inherited_stdio() -> Stack {
        Self::new(IoStream::Inherit, IoStream::Inherit)
    }

    pub fn with_output_capture() -> Stack {
        Self::new(IoStream::Capture, IoStream::Inherit)
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
        for (id, val) in &self.vars {
            if var_id == *id {
                return Ok(val.clone().with_span(span));
            }
        }

        Err(ShellError::VariableNotFoundAtRuntime { span })
    }

    pub fn get_var_with_origin(&self, var_id: VarId, span: Span) -> Result<Value, ShellError> {
        for (id, val) in &self.vars {
            if var_id == *id {
                return Ok(val.clone());
            }
        }

        if var_id == NU_VARIABLE_ID || var_id == ENV_VARIABLE_ID {
            return Err(ShellError::GenericError {
                error: "Built-in variables `$env` and `$nu` have no metadata".into(),
                msg: "no metadata available".into(),
                span: Some(span),
                help: None,
                inner: vec![],
            });
        }

        Err(ShellError::VariableNotFoundAtRuntime { span })
    }

    pub fn add_var(&mut self, var_id: VarId, value: Value) {
        //self.vars.insert(var_id, value);
        for (id, val) in &mut self.vars {
            if *id == var_id {
                *val = value;
                return;
            }
        }
        self.vars.push((var_id, value));
    }

    pub fn remove_var(&mut self, var_id: VarId) {
        for (idx, (id, _)) in self.vars.iter().enumerate() {
            if *id == var_id {
                self.vars.remove(idx);
                return;
            }
        }
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
                    scope.insert(last_overlay.into(), [(var, value)].into_iter().collect());
                }
            } else {
                self.env_vars.push(
                    [(last_overlay.into(), [(var, value)].into_iter().collect())]
                        .into_iter()
                        .collect(),
                );
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
            .ok_or_else(|| ShellError::NushellFailed {
                msg: "No active overlay".into(),
            })
    }

    pub fn captures_to_stack(&self, captures: Vec<(VarId, Value)>) -> Stack {
        // FIXME: this is probably slow
        let mut env_vars = self.env_vars.clone();
        env_vars.push(HashMap::new());

        Stack {
            vars: captures,
            env_vars,
            env_hidden: self.env_hidden.clone(),
            active_overlays: self.active_overlays.clone(),
            recursion_count: self.recursion_count,
            stdout: IoStream::Capture,
            stderr: self.stderr.clone(),
            parent_stdout: None,
            parent_stderr: None,
        }
    }

    pub fn gather_captures(&self, engine_state: &EngineState, captures: &[VarId]) -> Stack {
        let mut vars = vec![];

        let fake_span = Span::new(0, 0);

        for capture in captures {
            // Note: this assumes we have calculated captures correctly and that commands
            // that take in a var decl will manually set this into scope when running the blocks
            if let Ok(value) = self.get_var(*capture, fake_span) {
                vars.push((*capture, value));
            } else if let Some(const_val) = &engine_state.get_var(*capture).const_val {
                vars.push((*capture, const_val.clone()));
            }
        }

        let mut env_vars = self.env_vars.clone();
        env_vars.push(HashMap::new());

        Stack {
            vars,
            env_vars,
            env_hidden: self.env_hidden.clone(),
            active_overlays: self.active_overlays.clone(),
            recursion_count: self.recursion_count,
            stdout: self.stdout.clone(),
            stderr: self.stderr.clone(),
            parent_stdout: None,
            parent_stderr: None,
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

    pub fn remove_env_var(&mut self, engine_state: &EngineState, name: &str) -> bool {
        for scope in self.env_vars.iter_mut().rev() {
            for active_overlay in self.active_overlays.iter().rev() {
                if let Some(env_vars) = scope.get_mut(active_overlay) {
                    if env_vars.remove(name).is_some() {
                        return true;
                    }
                }
            }
        }

        for active_overlay in self.active_overlays.iter().rev() {
            if let Some(env_vars) = engine_state.env_vars.get(active_overlay) {
                if env_vars.get(name).is_some() {
                    if let Some(env_hidden) = self.env_hidden.get_mut(active_overlay) {
                        env_hidden.insert(name.into());
                    } else {
                        self.env_hidden
                            .insert(active_overlay.into(), [name.into()].into_iter().collect());
                    }

                    return true;
                }
            }
        }

        false
    }

    pub fn has_env_overlay(&self, name: &str, engine_state: &EngineState) -> bool {
        for scope in self.env_vars.iter().rev() {
            if scope.contains_key(name) {
                return true;
            }
        }

        engine_state.env_vars.contains_key(name)
    }

    pub fn is_overlay_active(&self, name: &str) -> bool {
        self.active_overlays.iter().any(|n| n == name)
    }

    pub fn add_overlay(&mut self, name: String) {
        self.active_overlays.retain(|o| o != &name);
        self.active_overlays.push(name);
    }

    pub fn remove_overlay(&mut self, name: &str) {
        self.active_overlays.retain(|o| o != name);
    }

    pub fn stdout(&self) -> &IoStream {
        &self.stdout
    }

    pub fn stderr(&self) -> &IoStream {
        &self.stderr
    }

    pub fn parent_stdout(&self) -> Option<&IoStream> {
        self.parent_stdout.as_ref()
    }

    pub fn parent_stderr(&self) -> Option<&IoStream> {
        self.parent_stderr.as_ref()
    }

    pub fn push_stdio(
        &mut self,
        stdout: Option<IoStream>,
        stderr: Option<IoStream>,
    ) -> StackIoGuard {
        let stdout = if let Some(stdout) = stdout {
            self.push_stdout(stdout)
        } else {
            self.parent_stdout.take()
        };

        let stderr = if let Some(stderr) = stderr {
            self.push_stderr(stderr)
        } else {
            self.parent_stderr.take()
        };

        StackIoGuard::new(self, stdout, stderr)
    }

    pub fn use_parent_stdio(&mut self) -> StackParentIoGuard {
        StackParentIoGuard::new(self)
    }

    pub fn reset_stdio(&mut self, stdout: IoStream, stderr: IoStream) {
        self.parent_stdout = None;
        self.parent_stderr = None;
        self.stdout = stdout;
        self.stderr = stderr;
    }

    fn push_stdout(&mut self, stdout: IoStream) -> Option<IoStream> {
        let stdout = mem::replace(&mut self.stdout, stdout);
        mem::replace(&mut self.parent_stdout, Some(stdout))
    }

    fn push_stderr(&mut self, stderr: IoStream) -> Option<IoStream> {
        let stderr = mem::replace(&mut self.stderr, stderr);
        mem::replace(&mut self.parent_stderr, Some(stderr))
    }
}

pub struct StackIoGuard<'a> {
    stack: &'a mut Stack,
    old_parent_stdout: Option<IoStream>,
    old_parent_stderr: Option<IoStream>,
}

impl<'a> StackIoGuard<'a> {
    fn new(
        stack: &'a mut Stack,
        old_parent_stdout: Option<IoStream>,
        old_parent_stderr: Option<IoStream>,
    ) -> Self {
        Self {
            stack,
            old_parent_stdout,
            old_parent_stderr,
        }
    }
}

impl<'a> Deref for StackIoGuard<'a> {
    type Target = Stack;

    fn deref(&self) -> &Self::Target {
        self.stack
    }
}

impl<'a> DerefMut for StackIoGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.stack
    }
}

impl Drop for StackIoGuard<'_> {
    fn drop(&mut self) {
        let old_stdout = self.old_parent_stdout.take();
        if let Some(stdout) = mem::replace(&mut self.parent_stdout, old_stdout) {
            self.stdout = stdout;
        }

        let old_stderr = self.old_parent_stderr.take();
        if let Some(stderr) = mem::replace(&mut self.parent_stderr, old_stderr) {
            self.stderr = stderr;
        }
    }
}

pub struct StackParentIoGuard<'a> {
    stack: &'a mut Stack,
    old_stdout: Option<IoStream>,
    old_stderr: Option<IoStream>,
}

impl<'a> StackParentIoGuard<'a> {
    fn new(stack: &'a mut Stack) -> Self {
        let old_stdout = Some(mem::replace(&mut stack.stdout, IoStream::Capture));

        let old_stderr = stack
            .parent_stderr
            .take()
            .map(|stderr| mem::replace(&mut stack.stderr, stderr));

        Self {
            stack,
            old_stdout,
            old_stderr,
        }
    }
}

impl<'a> Deref for StackParentIoGuard<'a> {
    type Target = Stack;

    fn deref(&self) -> &Self::Target {
        &*self.stack
    }
}

impl<'a> DerefMut for StackParentIoGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.stack
    }
}

impl Drop for StackParentIoGuard<'_> {
    fn drop(&mut self) {
        if let Some(stdout) = self.old_stdout.take() {
            self.stdout = stdout;
        }
        if let Some(stderr) = self.old_stderr.take() {
            self.stack.push_stderr(stderr);
        }
    }
}
