use crate::{
    evaluate::envvar::EnvVar,
    whole_stream_command::{whole_stream_command, Command},
};
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_parser::ParserScope;
use nu_protocol::{hir::Block, Signature, SignatureRegistry, Value};
use nu_source::Spanned;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Scope {
    frames: Arc<parking_lot::Mutex<Vec<ScopeFrame>>>,
}

impl Default for Scope {
    fn default() -> Self {
        Self::new()
    }
}

impl Scope {
    pub fn new() -> Scope {
        Scope {
            frames: Arc::new(parking_lot::Mutex::new(vec![ScopeFrame::new()])),
        }
    }

    pub fn get_command(&self, name: &str) -> Option<Command> {
        for frame in self.frames.lock().iter().rev() {
            if let Some(command) = frame.get_command(name) {
                return Some(command);
            }
        }

        None
    }

    pub fn get_aliases(&self) -> IndexMap<String, Vec<Spanned<String>>> {
        let mut output: IndexMap<String, Vec<Spanned<String>>> = IndexMap::new();

        for frame in self.frames.lock().iter().rev() {
            for v in &frame.aliases {
                if !output.contains_key(v.0) {
                    output.insert(v.0.clone(), v.1.clone());
                }
            }
        }

        output.sorted_by(|k1, _v1, k2, _v2| k1.cmp(k2)).collect()
    }

    pub fn get_commands(&self) -> IndexMap<String, Signature> {
        let mut output: IndexMap<String, Signature> = IndexMap::new();

        for frame in self.frames.lock().iter().rev() {
            for (name, command) in &frame.commands {
                if !output.contains_key(name) {
                    let mut sig = command.signature();
                    // don't show --help and -h in the command arguments for $scope.commands
                    sig.remove_named("help");
                    output.insert(name.clone(), sig);
                }
            }
        }

        output.sorted_by(|k1, _v1, k2, _v2| k1.cmp(k2)).collect()
    }

    pub fn get_commands_info(&self) -> IndexMap<String, Command> {
        let mut output: IndexMap<String, Command> = IndexMap::new();

        for frame in self.frames.lock().iter().rev() {
            for (name, command) in &frame.commands {
                if !output.contains_key(name) {
                    output.insert(name.clone(), command.clone());
                }
            }
        }

        output.sorted_by(|k1, _v1, k2, _v2| k1.cmp(k2)).collect()
    }

    pub fn get_variable_names(&self) -> Vec<String> {
        self.get_vars().iter().map(|(k, _)| k.to_string()).collect()
    }

    pub fn get_vars(&self) -> IndexMap<String, Value> {
        //FIXME: should this be an iterator?
        let mut output: IndexMap<String, Value> = IndexMap::new();

        for frame in self.frames.lock().iter().rev() {
            for v in &frame.vars {
                if !output.contains_key(v.0) {
                    output.insert(v.0.clone(), v.1.clone());
                }
            }
        }

        output.sorted_by(|k1, _v1, k2, _v2| k1.cmp(k2)).collect()
    }

    pub fn get_aliases_with_name(&self, name: &str) -> Option<Vec<Vec<Spanned<String>>>> {
        let aliases: Vec<_> = self
            .frames
            .lock()
            .iter()
            .rev()
            .filter_map(|frame| frame.aliases.get(name).cloned())
            .collect();
        if aliases.is_empty() {
            None
        } else {
            Some(aliases)
        }
    }

    pub fn get_custom_commands_with_name(&self, name: &str) -> Option<Vec<Arc<Block>>> {
        let custom_commands: Vec<_> = self
            .frames
            .lock()
            .iter()
            .rev()
            .filter_map(|frame| frame.custom_commands.get(name).cloned())
            .collect();

        if custom_commands.is_empty() {
            None
        } else {
            Some(custom_commands)
        }
    }

    pub fn add_command(&self, name: String, command: Command) {
        // Note: this is assumed to always be true, as there is always a global top frame
        if let Some(frame) = self.frames.lock().last_mut() {
            frame.add_command(name, command)
        }
    }

    pub fn get_alias_names(&self) -> Vec<String> {
        let mut names = vec![];

        for frame in self.frames.lock().iter() {
            let mut frame_command_names = frame.get_alias_names();
            names.append(&mut frame_command_names);
        }

        // Sort needs to happen first because dedup works on consecutive dupes only
        names.sort();
        names.dedup();

        names
    }

    pub fn get_command_names(&self) -> Vec<String> {
        let mut names = vec![];

        for frame in self.frames.lock().iter() {
            let mut frame_command_names = frame.get_command_names();
            frame_command_names.extend(frame.get_alias_names());
            frame_command_names.extend(frame.get_custom_command_names());
            names.append(&mut frame_command_names);
        }

        // Sort needs to happen first because dedup works on consecutive dupes only
        names.sort();
        names.dedup();

        names
    }

    pub fn len(&self) -> usize {
        self.frames.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.lock().is_empty()
    }

    fn has_cmd_helper(&self, name: &str, f: fn(&ScopeFrame, &str) -> bool) -> bool {
        self.frames.lock().iter().any(|frame| f(frame, name))
    }

    pub fn has_command(&self, name: &str) -> bool {
        self.has_cmd_helper(name, ScopeFrame::has_command)
    }

    pub fn has_custom_command(&self, name: &str) -> bool {
        self.has_cmd_helper(name, ScopeFrame::has_custom_command)
    }

    pub fn has_alias(&self, name: &str) -> bool {
        self.has_cmd_helper(name, ScopeFrame::has_alias)
    }

    pub fn expect_command(&self, name: &str) -> Result<Command, ShellError> {
        if let Some(c) = self.get_command(name) {
            Ok(c)
        } else {
            Err(ShellError::untagged_runtime_error(format!(
                "Missing command '{}'",
                name
            )))
        }
    }

    // This is used for starting processes, keep it string -> string
    pub fn get_env_vars(&self) -> IndexMap<String, String> {
        //FIXME: should this be an iterator?
        let mut output = IndexMap::new();

        for frame in self.frames.lock().iter().rev() {
            for v in &frame.env {
                if !output.contains_key(v.0) {
                    output.insert(v.0.clone(), v.1.clone());
                }
            }
        }

        output
            .into_iter()
            .filter_map(|(k, v)| match v {
                EnvVar::Proper(s) => Some((k, s)),
                EnvVar::Nothing => None,
            })
            .collect()
    }

    pub fn get_env(&self, name: &str) -> Option<String> {
        for frame in self.frames.lock().iter().rev() {
            if let Some(v) = frame.env.get(name) {
                return match v {
                    EnvVar::Proper(string) => Some(string.clone()),
                    EnvVar::Nothing => None,
                };
            }
        }

        None
    }

    pub fn get_var(&self, name: &str) -> Option<Value> {
        for frame in self.frames.lock().iter().rev() {
            if let Some(v) = frame.vars.get(name) {
                return Some(v.clone());
            }
        }

        None
    }

    pub fn add_var(&self, name: impl Into<String>, value: Value) {
        if let Some(frame) = self.frames.lock().last_mut() {
            frame.vars.insert(name.into(), value);
        }
    }

    pub fn add_vars(&self, vars: &IndexMap<String, Value>) {
        if let Some(frame) = self.frames.lock().last_mut() {
            frame
                .vars
                .extend(vars.iter().map(|(s, v)| (s.clone(), v.clone())))
        }
    }

    pub fn add_env_var(&self, name: impl Into<String>, value: impl Into<EnvVar>) {
        if let Some(frame) = self.frames.lock().last_mut() {
            frame.env.insert(name.into(), value.into());
        }
    }

    pub fn remove_env_var(&self, name: impl Into<String>) -> Option<String> {
        if let Some(frame) = self.frames.lock().last_mut() {
            if let Some(val) = frame.env.remove_entry(&name.into()) {
                return Some(val.0);
            }
        }
        None
    }

    pub fn add_env(&self, env_vars: IndexMap<String, EnvVar>) {
        if let Some(frame) = self.frames.lock().last_mut() {
            frame.env.extend(env_vars)
        }
    }

    pub fn add_env_to_base(&self, env_vars: IndexMap<String, EnvVar>) {
        if let Some(frame) = self.frames.lock().first_mut() {
            frame.env.extend(env_vars)
        }
    }

    pub fn add_env_var_to_base(&self, name: impl Into<String>, value: impl Into<EnvVar>) {
        if let Some(frame) = self.frames.lock().first_mut() {
            frame.env.insert(name.into(), value.into());
        }
    }

    pub fn set_exit_scripts(&self, scripts: Vec<String>) {
        if let Some(frame) = self.frames.lock().last_mut() {
            frame.exitscripts = scripts
        }
    }

    pub fn enter_scope_with_tag(&self, tag: String) {
        self.frames.lock().push(ScopeFrame::with_tag(tag));
    }

    //Removes the scopeframe with tag.
    pub fn exit_scope_with_tag(&self, tag: &str) {
        let mut frames = self.frames.lock();
        let tag = Some(tag);
        if let Some(i) = frames.iter().rposition(|f| f.tag.as_deref() == tag) {
            frames.remove(i);
        }
    }

    pub fn get_exitscripts_of_frame_with_tag(&self, tag: &str) -> Option<Vec<String>> {
        let frames = self.frames.lock();
        let tag = Some(tag);
        frames.iter().find_map(|f| {
            if f.tag.as_deref() == tag {
                Some(f.exitscripts.clone())
            } else {
                None
            }
        })
    }

    pub fn get_frame_with_tag(&self, tag: &str) -> Option<ScopeFrame> {
        let frames = self.frames.lock();
        let tag = Some(tag);
        frames.iter().rev().find_map(|f| {
            if f.tag.as_deref() == tag {
                Some(f.clone())
            } else {
                None
            }
        })
    }

    pub fn update_frame_with_tag(&self, frame: ScopeFrame, tag: &str) -> Result<(), ShellError> {
        let mut frames = self.frames.lock();
        let tag = Some(tag);
        for f in frames.iter_mut().rev() {
            if f.tag.as_deref() == tag {
                *f = frame;
                return Ok(());
            }
        }

        // Frame not found, return err
        Err(ShellError::untagged_runtime_error(format!(
            "Can't update frame with tag {:?}. No such frame present!",
            tag
        )))
    }
}

impl SignatureRegistry for Scope {
    fn names(&self) -> Vec<String> {
        self.get_command_names()
    }

    fn has(&self, name: &str) -> bool {
        self.get_signature(name).is_some()
    }

    fn get(&self, name: &str) -> Option<Signature> {
        self.get_signature(name)
    }

    fn clone_box(&self) -> Box<dyn SignatureRegistry> {
        Box::new(self.clone())
    }
}

impl ParserScope for Scope {
    fn get_signature(&self, name: &str) -> Option<Signature> {
        self.get_command(name).map(|x| x.signature())
    }

    fn has_signature(&self, name: &str) -> bool {
        self.get_command(name).is_some()
    }

    fn add_definition(&self, block: Arc<Block>) {
        if let Some(frame) = self.frames.lock().last_mut() {
            let name = block.params.name.clone();
            frame.custom_commands.insert(name.clone(), block.clone());
            frame.commands.insert(name, whole_stream_command(block));
        }
    }

    fn get_definitions(&self) -> Vec<Arc<Block>> {
        let mut blocks = vec![];
        if let Some(frame) = self.frames.lock().last() {
            for (_, custom_command) in &frame.custom_commands {
                blocks.push(custom_command.clone());
            }
        }
        blocks
    }

    fn get_alias(&self, name: &str) -> Option<Vec<Spanned<String>>> {
        for frame in self.frames.lock().iter().rev() {
            if let Some(x) = frame.aliases.get(name) {
                return Some(x.clone());
            }
        }
        None
    }

    fn add_alias(&self, name: &str, replacement: Vec<Spanned<String>>) {
        // Note: this is assumed to always be true, as there is always a global top frame
        if let Some(frame) = self.frames.lock().last_mut() {
            frame.aliases.insert(name.to_string(), replacement);
        }
    }

    fn remove_alias(&self, name: &str) {
        if let Some(frame) = self.frames.lock().last_mut() {
            frame.aliases.remove(name);
        }
    }

    fn enter_scope(&self) {
        self.frames.lock().push(ScopeFrame::new());
    }

    fn exit_scope(&self) {
        self.frames.lock().pop();
    }
}

/// An evaluation scope. Scopes map variable names to Values and aid in evaluating blocks and expressions.
#[derive(Debug, Clone)]
pub struct ScopeFrame {
    pub vars: IndexMap<String, Value>,
    pub env: IndexMap<String, EnvVar>,
    pub commands: IndexMap<String, Command>,
    pub custom_commands: IndexMap<String, Arc<Block>>,
    pub aliases: IndexMap<String, Vec<Spanned<String>>>,
    ///Optional tag to better identify this scope frame later
    pub tag: Option<String>,
    pub exitscripts: Vec<String>,
}

impl Default for ScopeFrame {
    fn default() -> Self {
        ScopeFrame::new()
    }
}

impl ScopeFrame {
    pub fn has_command(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    pub fn has_custom_command(&self, name: &str) -> bool {
        self.custom_commands.contains_key(name)
    }

    pub fn has_alias(&self, name: &str) -> bool {
        self.aliases.contains_key(name)
    }

    pub fn get_alias_names(&self) -> Vec<String> {
        self.aliases.keys().map(|x| x.to_string()).collect()
    }

    pub fn get_command_names(&self) -> Vec<String> {
        self.commands.keys().map(|x| x.to_string()).collect()
    }

    pub fn get_custom_command_names(&self) -> Vec<String> {
        self.custom_commands.keys().map(|x| x.to_string()).collect()
    }

    pub fn add_command(&mut self, name: String, command: Command) {
        self.commands.insert(name, command);
    }

    pub fn get_command(&self, name: &str) -> Option<Command> {
        self.commands.get(name).cloned()
    }

    pub fn new() -> ScopeFrame {
        ScopeFrame {
            vars: IndexMap::new(),
            env: IndexMap::new(),
            commands: IndexMap::new(),
            custom_commands: IndexMap::new(),
            aliases: IndexMap::new(),
            tag: None,
            exitscripts: Vec::new(),
        }
    }

    pub fn with_tag(tag: String) -> ScopeFrame {
        let mut scope = ScopeFrame::new();
        scope.tag = Some(tag);

        scope
    }
}
