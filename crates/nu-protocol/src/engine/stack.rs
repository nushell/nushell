use crate::{
    engine::{
        EngineState, Redirection, StackCallArgGuard, StackCaptureGuard, StackIoGuard, StackOutDest,
        DEFAULT_OVERLAY_NAME,
    },
    OutDest, ShellError, Span, Value, VarId, ENV_VARIABLE_ID, NU_VARIABLE_ID,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
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
    pub parent_stack: Option<Arc<Stack>>,
    /// Variables that have been deleted (this is used to hide values from parent stack lookups)
    pub parent_deletions: Vec<VarId>,
    pub(crate) out_dest: StackOutDest,
}

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

impl Stack {
    /// Create a new stack.
    ///
    /// stdout and stderr will be set to [`OutDest::Inherit`]. So, if the last command is an external command,
    /// then its output will be forwarded to the terminal/stdio streams.
    ///
    /// Use [`Stack::capture`] afterwards if you need to evaluate an expression to a [`Value`](crate::Value)
    /// (as opposed to a [`PipelineData`](crate::PipelineData)).
    pub fn new() -> Self {
        Self {
            vars: Vec::new(),
            env_vars: Vec::new(),
            env_hidden: HashMap::new(),
            active_overlays: vec![DEFAULT_OVERLAY_NAME.to_string()],
            recursion_count: 0,
            parent_stack: None,
            parent_deletions: vec![],
            out_dest: StackOutDest::new(),
        }
    }

    /// Unwrap a uniquely-owned stack.
    ///
    /// In debug mode, this panics if there are multiple references.
    /// In production this will instead clone the underlying stack.
    pub fn unwrap_unique(stack_arc: Arc<Stack>) -> Stack {
        // If you hit an error here, it's likely that you created an extra
        // Arc pointing to the stack somewhere. Make sure that it gets dropped before
        // getting here!
        Arc::try_unwrap(stack_arc).unwrap_or_else(|arc| {
            // in release mode, we clone the stack, but this can lead to
            // major performance issues, so we should avoid it
            debug_assert!(false, "More than one stack reference remaining!");
            (*arc).clone()
        })
    }

    /// Create a new child stack from a parent.
    ///
    /// Changes from this child can be merged back into the parent with
    /// Stack::with_changes_from_child
    pub fn with_parent(parent: Arc<Stack>) -> Stack {
        Stack {
            // here we are still cloning environment variable-related information
            env_vars: parent.env_vars.clone(),
            env_hidden: parent.env_hidden.clone(),
            active_overlays: parent.active_overlays.clone(),
            recursion_count: parent.recursion_count,
            vars: vec![],
            parent_deletions: vec![],
            out_dest: parent.out_dest.clone(),
            parent_stack: Some(parent),
        }
    }

    /// Take an Arc of a parent (assumed to be unique), and a child, and apply
    /// all the changes from a child back to the parent.
    ///
    /// Here it is assumed that child was created with a call to Stack::with_parent
    /// with parent
    pub fn with_changes_from_child(parent: Arc<Stack>, child: Stack) -> Stack {
        // we're going to drop the link to the parent stack on our new stack
        // so that we can unwrap the Arc as a unique reference
        //
        // This makes the new_stack be in a bit of a weird state, so we shouldn't call
        // any structs
        drop(child.parent_stack);
        let mut unique_stack = Stack::unwrap_unique(parent);

        unique_stack
            .vars
            .retain(|(var, _)| !child.parent_deletions.contains(var));
        for (var, value) in child.vars {
            unique_stack.add_var(var, value);
        }
        unique_stack.env_vars = child.env_vars;
        unique_stack.env_hidden = child.env_hidden;
        unique_stack.active_overlays = child.active_overlays;
        unique_stack
    }

    pub fn with_env(
        &mut self,
        env_vars: &[EnvVars],
        env_hidden: &HashMap<String, HashSet<String>>,
    ) {
        // Do not clone the environment if it hasn't changed
        if self.env_vars.iter().any(|scope| !scope.is_empty()) {
            env_vars.clone_into(&mut self.env_vars);
        }

        if !self.env_hidden.is_empty() {
            self.env_hidden.clone_from(env_hidden);
        }
    }

    /// Lookup a variable, returning None if it is not present
    fn lookup_var(&self, var_id: VarId) -> Option<Value> {
        for (id, val) in &self.vars {
            if var_id == *id {
                return Some(val.clone());
            }
        }

        if let Some(stack) = &self.parent_stack {
            if !self.parent_deletions.contains(&var_id) {
                return stack.lookup_var(var_id);
            }
        }
        None
    }

    /// Lookup a variable, erroring if it is not found
    ///
    /// The passed-in span will be used to tag the value
    pub fn get_var(&self, var_id: VarId, span: Span) -> Result<Value, ShellError> {
        match self.lookup_var(var_id) {
            Some(v) => Ok(v.with_span(span)),
            None => Err(ShellError::VariableNotFoundAtRuntime { span }),
        }
    }

    /// Lookup a variable, erroring if it is not found
    ///
    /// While the passed-in span will be used for errors, the returned value
    /// has the span from where it was originally defined
    pub fn get_var_with_origin(&self, var_id: VarId, span: Span) -> Result<Value, ShellError> {
        match self.lookup_var(var_id) {
            Some(v) => Ok(v),
            None => {
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
        }
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
                break;
            }
        }
        // even if we did have it in the original layer, we need to make sure to remove it here
        // as well (since the previous update might have simply hid the parent value)
        if self.parent_stack.is_some() {
            self.parent_deletions.push(var_id);
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
        self.captures_to_stack_preserve_out_dest(captures).capture()
    }

    pub fn captures_to_stack_preserve_out_dest(&self, captures: Vec<(VarId, Value)>) -> Stack {
        // FIXME: this is probably slow
        let mut env_vars = self.env_vars.clone();
        env_vars.push(HashMap::new());

        Stack {
            vars: captures,
            env_vars,
            env_hidden: self.env_hidden.clone(),
            active_overlays: self.active_overlays.clone(),
            recursion_count: self.recursion_count,
            parent_stack: None,
            parent_deletions: vec![],
            out_dest: self.out_dest.clone(),
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
            parent_stack: None,
            parent_deletions: vec![],
            out_dest: self.out_dest.clone(),
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

    /// Returns the [`OutDest`] to use for the current command's stdout.
    ///
    /// This will be the pipe redirection if one is set,
    /// otherwise it will be the current file redirection,
    /// otherwise it will be the process's stdout indicated by [`OutDest::Inherit`].
    pub fn stdout(&self) -> &OutDest {
        self.out_dest.stdout()
    }

    /// Returns the [`OutDest`] to use for the current command's stderr.
    ///
    /// This will be the pipe redirection if one is set,
    /// otherwise it will be the current file redirection,
    /// otherwise it will be the process's stderr indicated by [`OutDest::Inherit`].
    pub fn stderr(&self) -> &OutDest {
        self.out_dest.stderr()
    }

    /// Returns the [`OutDest`] of the pipe redirection applied to the current command's stdout.
    pub fn pipe_stdout(&self) -> Option<&OutDest> {
        self.out_dest.pipe_stdout.as_ref()
    }

    /// Returns the [`OutDest`] of the pipe redirection applied to the current command's stderr.
    pub fn pipe_stderr(&self) -> Option<&OutDest> {
        self.out_dest.pipe_stderr.as_ref()
    }

    /// Temporarily set the pipe stdout redirection to [`OutDest::Capture`].
    ///
    /// This is used before evaluating an expression into a `Value`.
    pub fn start_capture(&mut self) -> StackCaptureGuard {
        StackCaptureGuard::new(self)
    }

    /// Temporarily use the output redirections in the parent scope.
    ///
    /// This is used before evaluating an argument to a call.
    pub fn use_call_arg_out_dest(&mut self) -> StackCallArgGuard {
        StackCallArgGuard::new(self)
    }

    /// Temporarily apply redirections to stdout and/or stderr.
    pub fn push_redirection(
        &mut self,
        stdout: Option<Redirection>,
        stderr: Option<Redirection>,
    ) -> StackIoGuard {
        StackIoGuard::new(self, stdout, stderr)
    }

    /// Mark stdout for the last command as [`OutDest::Capture`].
    ///
    /// This will irreversibly alter the output redirections, and so it only makes sense to use this on an owned `Stack`
    /// (which is why this function does not take `&mut self`).
    ///
    /// See [`Stack::start_capture`] which can temporarily set stdout as [`OutDest::Capture`] for a mutable `Stack` reference.
    pub fn capture(mut self) -> Self {
        self.out_dest.pipe_stdout = Some(OutDest::Capture);
        self.out_dest.pipe_stderr = None;
        self
    }

    /// Clears any pipe and file redirections and resets stdout and stderr to [`OutDest::Inherit`].
    ///
    /// This will irreversibly reset the output redirections, and so it only makes sense to use this on an owned `Stack`
    /// (which is why this function does not take `&mut self`).
    pub fn reset_out_dest(mut self) -> Self {
        self.out_dest = StackOutDest::new();
        self
    }

    /// Clears any pipe redirections, keeping the current stdout and stderr.
    ///
    /// This will irreversibly reset some of the output redirections, and so it only makes sense to use this on an owned `Stack`
    /// (which is why this function does not take `&mut self`).
    pub fn reset_pipes(mut self) -> Self {
        self.out_dest.pipe_stdout = None;
        self.out_dest.pipe_stderr = None;
        self
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use crate::{engine::EngineState, Span, Value};

    use super::Stack;

    const ZERO_SPAN: Span = Span { start: 0, end: 0 };
    fn string_value(s: &str) -> Value {
        Value::String {
            val: s.to_string(),
            internal_span: ZERO_SPAN,
        }
    }

    #[test]
    fn test_children_see_inner_values() {
        let mut original = Stack::new();
        original.add_var(0, string_value("hello"));

        let cloned = Stack::with_parent(Arc::new(original));
        assert_eq!(cloned.get_var(0, ZERO_SPAN), Ok(string_value("hello")));
    }

    #[test]
    fn test_children_dont_see_deleted_values() {
        let mut original = Stack::new();
        original.add_var(0, string_value("hello"));

        let mut cloned = Stack::with_parent(Arc::new(original));
        cloned.remove_var(0);

        assert_eq!(
            cloned.get_var(0, ZERO_SPAN),
            Err(crate::ShellError::VariableNotFoundAtRuntime { span: ZERO_SPAN })
        );
    }

    #[test]
    fn test_children_changes_override_parent() {
        let mut original = Stack::new();
        original.add_var(0, string_value("hello"));

        let mut cloned = Stack::with_parent(Arc::new(original));
        cloned.add_var(0, string_value("there"));
        assert_eq!(cloned.get_var(0, ZERO_SPAN), Ok(string_value("there")));

        cloned.remove_var(0);
        // the underlying value shouldn't magically re-appear
        assert_eq!(
            cloned.get_var(0, ZERO_SPAN),
            Err(crate::ShellError::VariableNotFoundAtRuntime { span: ZERO_SPAN })
        );
    }
    #[test]
    fn test_children_changes_persist_in_offspring() {
        let mut original = Stack::new();
        original.add_var(0, string_value("hello"));

        let mut cloned = Stack::with_parent(Arc::new(original));
        cloned.add_var(1, string_value("there"));

        cloned.remove_var(0);
        let cloned = Stack::with_parent(Arc::new(cloned));

        assert_eq!(
            cloned.get_var(0, ZERO_SPAN),
            Err(crate::ShellError::VariableNotFoundAtRuntime { span: ZERO_SPAN })
        );

        assert_eq!(cloned.get_var(1, ZERO_SPAN), Ok(string_value("there")));
    }

    #[test]
    fn test_merging_children_back_to_parent() {
        let mut original = Stack::new();
        let engine_state = EngineState::new();
        original.add_var(0, string_value("hello"));

        let original_arc = Arc::new(original);
        let mut cloned = Stack::with_parent(original_arc.clone());
        cloned.add_var(1, string_value("there"));

        cloned.remove_var(0);

        cloned.add_env_var("ADDED_IN_CHILD".to_string(), string_value("New Env Var"));

        let original = Stack::with_changes_from_child(original_arc, cloned);

        assert_eq!(
            original.get_var(0, ZERO_SPAN),
            Err(crate::ShellError::VariableNotFoundAtRuntime { span: ZERO_SPAN })
        );

        assert_eq!(original.get_var(1, ZERO_SPAN), Ok(string_value("there")));

        assert_eq!(
            original.get_env_var(&engine_state, "ADDED_IN_CHILD"),
            Some(string_value("New Env Var")),
        );
    }
}
