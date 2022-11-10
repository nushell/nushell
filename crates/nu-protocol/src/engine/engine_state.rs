use super::{Command, EnvVars, OverlayFrame, ScopeFrame, Stack, Visibility, DEFAULT_OVERLAY_NAME};
use crate::Value;
use crate::{
    ast::Block, AliasId, BlockId, Config, DeclId, Example, Module, ModuleId, OverlayId, ShellError,
    Signature, Span, Type, VarId, Variable,
};
use core::panic;
use std::borrow::Borrow;
use std::path::Path;
use std::path::PathBuf;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{
        atomic::{AtomicBool, AtomicU32},
        Arc, Mutex,
    },
};

static PWD_ENV: &str = "PWD";

// TODO: move to different file? where?
/// An operation to be performed with the current buffer of the interactive shell.
#[derive(Clone)]
pub enum ReplOperation {
    Append(String),
    Insert(String),
    Replace(String),
}

/// The core global engine state. This includes all global definitions as well as any global state that
/// will persist for the whole session.
///
/// Declarations, variables, blocks, and other forms of data are held in the global state and referenced
/// elsewhere using their IDs. These IDs are simply their index into the global state. This allows us to
/// more easily handle creating blocks, binding variables and callsites, and more, because each of these
/// will refer to the corresponding IDs rather than their definitions directly. At runtime, this means
/// less copying and smaller structures.
///
/// Note that the runtime stack is not part of this global state. Runtime stacks are handled differently,
/// but they also rely on using IDs rather than full definitions.
///
/// A note on implementation:
///
/// Much of the global definitions are built on the Bodil's 'im' crate. This gives us a way of working with
/// lists of definitions in a way that is very cheap to access, while also allowing us to update them at
/// key points in time (often, the transition between parsing and evaluation).
///
/// Over the last two years we tried a few different approaches to global state like this. I'll list them
/// here for posterity, so we can more easily know how we got here:
///
/// * `Rc` - Rc is cheap, but not thread-safe. The moment we wanted to work with external processes, we
/// needed a way send to stdin/stdout. In Rust, the current practice is to spawn a thread to handle both.
/// These threads would need access to the global state, as they'll need to process data as it streams out
/// of the data pipeline. Because Rc isn't thread-safe, this breaks.
///
/// * `Arc` - Arc is the thread-safe version of the above. Often Arc is used in combination with a Mutex or
/// RwLock, but you can use Arc by itself. We did this a few places in the original Nushell. This *can* work
/// but because of Arc's nature of not allowing mutation if there's a second copy of the Arc around, this
/// ultimately becomes limiting.
///
/// * `Arc` + `Mutex/RwLock` - the standard practice for thread-safe containers. Unfortunately, this would
/// have meant we would incur a lock penalty every time we needed to access any declaration or block. As we
/// would be reading far more often than writing, it made sense to explore solutions that favor large amounts
/// of reads.
///
/// * `im` - the `im` crate was ultimately chosen because it has some very nice properties: it gives the
/// ability to cheaply clone these structures, which is nice as EngineState may need to be cloned a fair bit
/// to follow ownership rules for closures and iterators. It also is cheap to access. Favoring reads here fits
/// more closely to what we need with Nushell. And, of course, it's still thread-safe, so we get the same
/// benefits as above.
///
#[derive(Clone)]
pub struct EngineState {
    files: Vec<(String, usize, usize)>,
    file_contents: Vec<(Vec<u8>, usize, usize)>,
    vars: Vec<Variable>,
    decls: Vec<Box<dyn Command + 'static>>,
    aliases: Vec<Vec<Span>>,
    blocks: Vec<Block>,
    modules: Vec<Module>,
    pub scope: ScopeFrame,
    pub ctrlc: Option<Arc<AtomicBool>>,
    pub env_vars: EnvVars,
    pub previous_env_vars: HashMap<String, Value>,
    pub config: Config,
    pub pipeline_externals_state: Arc<(AtomicU32, AtomicU32)>,
    pub repl_buffer_state: Arc<Mutex<Option<String>>>,
    pub repl_operation_queue: Arc<Mutex<VecDeque<ReplOperation>>>,
    #[cfg(feature = "plugin")]
    pub plugin_signatures: Option<PathBuf>,
    #[cfg(not(windows))]
    sig_quit: Option<Arc<AtomicBool>>,
    config_path: HashMap<String, PathBuf>,
    pub history_session_id: i64,
}

pub const NU_VARIABLE_ID: usize = 0;
pub const IN_VARIABLE_ID: usize = 1;
pub const ENV_VARIABLE_ID: usize = 2;
// NOTE: If you add more to this list, make sure to update the > checks based on the last in the list

impl EngineState {
    pub fn new() -> Self {
        Self {
            files: vec![],
            file_contents: vec![],
            vars: vec![
                Variable::new(Span::new(0, 0), Type::Any, false),
                Variable::new(Span::new(0, 0), Type::Any, false),
                Variable::new(Span::new(0, 0), Type::Any, false),
                Variable::new(Span::new(0, 0), Type::Any, false),
                Variable::new(Span::new(0, 0), Type::Any, false),
            ],
            decls: vec![],
            aliases: vec![],
            blocks: vec![],
            modules: vec![Module::new()],
            // make sure we have some default overlay:
            scope: ScopeFrame::with_empty_overlay(
                DEFAULT_OVERLAY_NAME.as_bytes().to_vec(),
                0,
                false,
            ),
            ctrlc: None,
            env_vars: EnvVars::from([(DEFAULT_OVERLAY_NAME.to_string(), HashMap::new())]),
            previous_env_vars: HashMap::new(),
            config: Config::default(),
            pipeline_externals_state: Arc::new((AtomicU32::new(0), AtomicU32::new(0))),
            repl_buffer_state: Arc::new(Mutex::new(None)),
            repl_operation_queue: Arc::new(Mutex::new(VecDeque::new())),
            #[cfg(feature = "plugin")]
            plugin_signatures: None,
            #[cfg(not(windows))]
            sig_quit: None,
            config_path: HashMap::new(),
            history_session_id: 0,
        }
    }

    /// Merges a `StateDelta` onto the current state. These deltas come from a system, like the parser, that
    /// creates a new set of definitions and visible symbols in the current scope. We make this transactional
    /// as there are times when we want to run the parser and immediately throw away the results (namely:
    /// syntax highlighting and completions).
    ///
    /// When we want to preserve what the parser has created, we can take its output (the `StateDelta`) and
    /// use this function to merge it into the global state.
    pub fn merge_delta(&mut self, mut delta: StateDelta) -> Result<(), ShellError> {
        // Take the mutable reference and extend the permanent state from the working set
        self.files.extend(delta.files);
        self.file_contents.extend(delta.file_contents);
        self.decls.extend(delta.decls);
        self.aliases.extend(delta.aliases);
        self.vars.extend(delta.vars);
        self.blocks.extend(delta.blocks);
        self.modules.extend(delta.modules);

        let first = delta.scope.remove(0);

        for (delta_name, delta_overlay) in first.clone().overlays {
            if let Some((_, existing_overlay)) = self
                .scope
                .overlays
                .iter_mut()
                .find(|(name, _)| name == &delta_name)
            {
                // Upating existing overlay
                for item in delta_overlay.decls.into_iter() {
                    existing_overlay.decls.insert(item.0, item.1);
                }
                for item in delta_overlay.vars.into_iter() {
                    existing_overlay.vars.insert(item.0, item.1);
                }
                for item in delta_overlay.aliases.into_iter() {
                    existing_overlay.aliases.insert(item.0, item.1);
                }
                for item in delta_overlay.modules.into_iter() {
                    existing_overlay.modules.insert(item.0, item.1);
                }

                existing_overlay
                    .visibility
                    .merge_with(delta_overlay.visibility);
            } else {
                // New overlay was added to the delta
                self.scope.overlays.push((delta_name, delta_overlay));
            }
        }

        let mut activated_ids = self.translate_overlay_ids(&first);

        let mut removed_ids = vec![];

        for name in &first.removed_overlays {
            if let Some(overlay_id) = self.find_overlay(name) {
                removed_ids.push(overlay_id);
            }
        }

        // Remove overlays removed in delta
        self.scope
            .active_overlays
            .retain(|id| !removed_ids.contains(id));

        // Move overlays activated in the delta to be first
        self.scope
            .active_overlays
            .retain(|id| !activated_ids.contains(id));
        self.scope.active_overlays.append(&mut activated_ids);

        #[cfg(feature = "plugin")]
        if delta.plugins_changed {
            let result = self.update_plugin_file();

            if result.is_ok() {
                delta.plugins_changed = false;
            }

            return result;
        }

        Ok(())
    }

    /// Merge the environment from the runtime Stack into the engine state
    pub fn merge_env(
        &mut self,
        stack: &mut Stack,
        cwd: impl AsRef<Path>,
    ) -> Result<(), ShellError> {
        for mut scope in stack.env_vars.drain(..) {
            for (overlay_name, mut env) in scope.drain() {
                if let Some(env_vars) = self.env_vars.get_mut(&overlay_name) {
                    // Updating existing overlay
                    for (k, v) in env.drain() {
                        if k == "config" {
                            self.config = v.clone().into_config().unwrap_or_default();
                        }

                        env_vars.insert(k, v);
                    }
                } else {
                    // Pushing a new overlay
                    self.env_vars.insert(overlay_name, env);
                }
            }
        }

        // TODO: better error
        std::env::set_current_dir(cwd)?;

        Ok(())
    }

    pub fn has_overlay(&self, name: &[u8]) -> bool {
        self.scope
            .overlays
            .iter()
            .any(|(overlay_name, _)| name == overlay_name)
    }

    pub fn active_overlay_ids(&self, removed_overlays: &[Vec<u8>]) -> Vec<OverlayId> {
        self.scope
            .active_overlays
            .iter()
            .filter(|id| !removed_overlays.contains(self.get_overlay_name(**id)))
            .copied()
            .collect()
    }

    pub fn active_overlays(&self, removed_overlays: &[Vec<u8>]) -> Vec<&OverlayFrame> {
        self.active_overlay_ids(removed_overlays)
            .iter()
            .map(|id| self.get_overlay(*id))
            .collect()
    }

    pub fn active_overlay_names(&self, removed_overlays: &[Vec<u8>]) -> Vec<&Vec<u8>> {
        self.active_overlay_ids(removed_overlays)
            .iter()
            .map(|id| self.get_overlay_name(*id))
            .collect()
    }

    /// Translate overlay IDs from other to IDs in self
    pub fn translate_overlay_ids(&self, other: &ScopeFrame) -> Vec<OverlayId> {
        let other_names = other.active_overlays.iter().map(|other_id| {
            &other
                .overlays
                .get(*other_id)
                .expect("internal error: missing overlay")
                .0
        });

        other_names
            .map(|other_name| {
                self.find_overlay(other_name)
                    .expect("internal error: missing overlay")
            })
            .collect()
    }

    pub fn last_overlay_name(&self, removed_overlays: &[Vec<u8>]) -> &Vec<u8> {
        self.active_overlay_names(removed_overlays)
            .last()
            .expect("internal error: no active overlays")
    }

    pub fn last_overlay(&self, removed_overlays: &[Vec<u8>]) -> &OverlayFrame {
        self.active_overlay_ids(removed_overlays)
            .last()
            .map(|id| self.get_overlay(*id))
            .expect("internal error: no active overlays")
    }

    pub fn get_overlay_name(&self, overlay_id: OverlayId) -> &Vec<u8> {
        &self
            .scope
            .overlays
            .get(overlay_id)
            .expect("internal error: missing overlay")
            .0
    }

    pub fn get_overlay(&self, overlay_id: OverlayId) -> &OverlayFrame {
        &self
            .scope
            .overlays
            .get(overlay_id)
            .expect("internal error: missing overlay")
            .1
    }

    pub fn render_env_vars(&self) -> HashMap<&String, &Value> {
        let mut result = HashMap::new();

        for overlay_name in self.active_overlay_names(&[]) {
            let name = String::from_utf8_lossy(overlay_name);
            if let Some(env_vars) = self.env_vars.get(name.as_ref()) {
                result.extend(env_vars);
            }
        }

        result
    }

    pub fn add_env_var(&mut self, name: String, val: Value) {
        let overlay_name = String::from_utf8_lossy(self.last_overlay_name(&[])).to_string();

        if let Some(env_vars) = self.env_vars.get_mut(&overlay_name) {
            env_vars.insert(name, val);
        } else {
            self.env_vars
                .insert(overlay_name, HashMap::from([(name, val)]));
        }
    }

    pub fn get_env_var(&self, name: &str) -> Option<&Value> {
        for overlay_id in self.scope.active_overlays.iter().rev() {
            let overlay_name = String::from_utf8_lossy(self.get_overlay_name(*overlay_id));
            if let Some(env_vars) = self.env_vars.get(overlay_name.as_ref()) {
                if let Some(val) = env_vars.get(name) {
                    return Some(val);
                }
            }
        }

        None
    }

    #[cfg(feature = "plugin")]
    pub fn update_plugin_file(&self) -> Result<(), ShellError> {
        use std::io::Write;

        // Updating the signatures plugin file with the added signatures
        self.plugin_signatures
            .as_ref()
            .ok_or_else(|| ShellError::PluginFailedToLoad("Plugin file not found".into()))
            .and_then(|plugin_path| {
                // Always create the file, which will erase previous signatures
                std::fs::File::create(plugin_path.as_path())
                    .map_err(|err| ShellError::PluginFailedToLoad(err.to_string()))
            })
            .and_then(|mut plugin_file| {
                // Plugin definitions with parsed signature
                self.plugin_decls().try_for_each(|decl| {
                    // A successful plugin registration already includes the plugin filename
                    // No need to check the None option
                    let (path, shell) = decl.is_plugin().expect("plugin should have file name");
                    let mut file_name = path
                        .to_str()
                        .expect("path was checked during registration as a str")
                        .to_string();

                    // Fix files or folders with quotes
                    if file_name.contains('\'')
                        || file_name.contains('"')
                        || file_name.contains(' ')
                    {
                        file_name = format!("`{}`", file_name);
                    }

                    serde_json::to_string_pretty(&decl.signature())
                        .map(|signature| {
                            // Extracting the possible path to the shell used to load the plugin
                            let shell_str = match shell {
                                Some(path) => format!(
                                    "-s {}",
                                    path.to_str().expect(
                                        "shell path was checked during registration as a str"
                                    )
                                ),
                                None => "".into(),
                            };

                            // Each signature is stored in the plugin file with the shell and signature
                            // This information will be used when loading the plugin
                            // information when nushell starts
                            format!("register {} {} {}\n\n", file_name, shell_str, signature)
                        })
                        .map_err(|err| ShellError::PluginFailedToLoad(err.to_string()))
                        .and_then(|line| {
                            plugin_file
                                .write_all(line.as_bytes())
                                .map_err(|err| ShellError::PluginFailedToLoad(err.to_string()))
                        })
                        .and_then(|_| {
                            plugin_file.flush().map_err(|err| {
                                ShellError::GenericError(
                                    "Error flushing plugin file".to_string(),
                                    format! {"{}", err},
                                    None,
                                    None,
                                    Vec::new(),
                                )
                            })
                        })
                })
            })
    }

    pub fn num_files(&self) -> usize {
        self.files.len()
    }

    pub fn num_vars(&self) -> usize {
        self.vars.len()
    }

    pub fn num_decls(&self) -> usize {
        self.decls.len()
    }

    pub fn num_aliases(&self) -> usize {
        self.aliases.len()
    }

    pub fn num_blocks(&self) -> usize {
        self.blocks.len()
    }

    pub fn num_modules(&self) -> usize {
        self.modules.len()
    }

    pub fn print_vars(&self) {
        for var in self.vars.iter().enumerate() {
            println!("var{}: {:?}", var.0, var.1);
        }
    }

    pub fn print_decls(&self) {
        for decl in self.decls.iter().enumerate() {
            println!("decl{}: {:?}", decl.0, decl.1.signature());
        }
    }

    pub fn print_blocks(&self) {
        for block in self.blocks.iter().enumerate() {
            println!("block{}: {:?}", block.0, block.1);
        }
    }

    pub fn print_contents(&self) {
        for (contents, _, _) in self.file_contents.iter() {
            let string = String::from_utf8_lossy(contents);
            println!("{}", string);
        }
    }

    pub fn find_decl(&self, name: &[u8], removed_overlays: &[Vec<u8>]) -> Option<DeclId> {
        let mut visibility: Visibility = Visibility::new();

        for overlay_frame in self.active_overlays(removed_overlays).iter().rev() {
            visibility.append(&overlay_frame.visibility);

            if let Some(decl_id) = overlay_frame.get_decl(name, &Type::Any) {
                if visibility.is_decl_id_visible(&decl_id) {
                    return Some(decl_id);
                }
            }
        }

        None
    }

    pub fn find_alias(&self, name: &[u8], removed_overlays: &[Vec<u8>]) -> Option<AliasId> {
        let mut visibility: Visibility = Visibility::new();

        for overlay_frame in self.active_overlays(removed_overlays).iter().rev() {
            visibility.append(&overlay_frame.visibility);

            if let Some(alias_id) = overlay_frame.aliases.get(name) {
                if visibility.is_alias_id_visible(alias_id) {
                    return Some(*alias_id);
                }
            }
        }

        None
    }

    #[cfg(feature = "plugin")]
    pub fn plugin_decls(&self) -> impl Iterator<Item = &Box<dyn Command + 'static>> {
        let mut unique_plugin_decls = HashMap::new();

        // Make sure there are no duplicate decls: Newer one overwrites the older one
        for decl in self.decls.iter().filter(|d| d.is_plugin().is_some()) {
            unique_plugin_decls.insert(decl.name(), decl);
        }

        let mut plugin_decls: Vec<(&str, &Box<dyn Command>)> =
            unique_plugin_decls.into_iter().collect();

        // Sort the plugins by name so we don't end up with a random plugin file each time
        plugin_decls.sort_by(|a, b| a.0.cmp(b.0));
        plugin_decls.into_iter().map(|(_, decl)| decl)
    }

    pub fn find_module(&self, name: &[u8], removed_overlays: &[Vec<u8>]) -> Option<ModuleId> {
        for overlay_frame in self.active_overlays(removed_overlays).iter().rev() {
            if let Some(module_id) = overlay_frame.modules.get(name) {
                return Some(*module_id);
            }
        }

        None
    }

    pub fn which_module_has_decl(&self, name: &[u8]) -> Option<&[u8]> {
        for (module_id, m) in self.modules.iter().enumerate() {
            if m.has_decl(name) {
                for overlay_frame in self.active_overlays(&[]).iter() {
                    let module_name = overlay_frame.modules.iter().find_map(|(key, &val)| {
                        if val == module_id {
                            Some(key)
                        } else {
                            None
                        }
                    });
                    if let Some(final_name) = module_name {
                        return Some(&final_name[..]);
                    }
                }
            }
        }
        None
    }

    pub fn find_overlay(&self, name: &[u8]) -> Option<OverlayId> {
        self.scope.find_overlay(name)
    }

    pub fn find_active_overlay(&self, name: &[u8]) -> Option<OverlayId> {
        self.scope.find_active_overlay(name)
    }

    pub fn find_commands_by_predicate(
        &self,
        predicate: impl Fn(&[u8]) -> bool,
    ) -> Vec<(Vec<u8>, Option<String>)> {
        let mut output = vec![];

        for overlay_frame in self.active_overlays(&[]).iter().rev() {
            for decl in &overlay_frame.decls {
                if overlay_frame.visibility.is_decl_id_visible(decl.1) && predicate(&decl.0 .0) {
                    let command = self.get_decl(*decl.1);
                    output.push((decl.0 .0.clone(), Some(command.usage().to_string())));
                }
            }
        }

        output
    }

    pub fn find_aliases_by_predicate(&self, predicate: impl Fn(&[u8]) -> bool) -> Vec<Vec<u8>> {
        let mut output = vec![];

        for overlay_frame in self.active_overlays(&[]).iter().rev() {
            for alias in &overlay_frame.aliases {
                if overlay_frame.visibility.is_alias_id_visible(alias.1) && predicate(alias.0) {
                    output.push(alias.0.clone());
                }
            }
        }

        output
    }

    pub fn get_span_contents(&self, span: &Span) -> &[u8] {
        for (contents, start, finish) in &self.file_contents {
            if span.start >= *start && span.end <= *finish {
                return &contents[(span.start - start)..(span.end - start)];
            }
        }
        &[0u8; 0]
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }

    pub fn set_config(&mut self, conf: &Config) {
        self.config = conf.clone();
    }

    pub fn get_var(&self, var_id: VarId) -> &Variable {
        self.vars
            .get(var_id)
            .expect("internal error: missing variable")
    }

    #[allow(clippy::borrowed_box)]
    pub fn get_decl(&self, decl_id: DeclId) -> &Box<dyn Command> {
        self.decls
            .get(decl_id)
            .expect("internal error: missing declaration")
    }

    pub fn get_alias(&self, alias_id: AliasId) -> &[Span] {
        self.aliases
            .get(alias_id)
            .expect("internal error: missing alias")
            .as_ref()
    }

    /// Get all IDs of all commands within scope, sorted by the commads' names
    pub fn get_decl_ids_sorted(&self, include_hidden: bool) -> impl Iterator<Item = DeclId> {
        let mut decls_map = HashMap::new();

        for overlay_frame in self.active_overlays(&[]) {
            let new_decls = if include_hidden {
                overlay_frame.decls.clone()
            } else {
                overlay_frame
                    .decls
                    .clone()
                    .into_iter()
                    .filter(|(_, id)| overlay_frame.visibility.is_decl_id_visible(id))
                    .collect()
            };

            decls_map.extend(new_decls);
        }

        let mut decls: Vec<(Vec<u8>, DeclId)> =
            decls_map.into_iter().map(|(v, k)| (v.0, k)).collect();

        decls.sort_by(|a, b| a.0.cmp(&b.0));
        decls.into_iter().map(|(_, id)| id)
    }

    /// Get signatures of all commands within scope.
    pub fn get_signatures(&self, include_hidden: bool) -> Vec<Signature> {
        self.get_decl_ids_sorted(include_hidden)
            .map(|id| {
                let decl = self.get_decl(id);

                (*decl).signature().update_from_command(decl.borrow())
            })
            .collect()
    }

    /// Get signatures of all commands within scope.
    ///
    /// In addition to signatures, it returns whether each command is:
    ///     a) a plugin
    ///     b) custom
    pub fn get_signatures_with_examples(
        &self,
        include_hidden: bool,
    ) -> Vec<(Signature, Vec<Example>, bool, bool)> {
        self.get_decl_ids_sorted(include_hidden)
            .map(|id| {
                let decl = self.get_decl(id);

                let signature = (*decl).signature().update_from_command(decl.borrow());

                (
                    signature,
                    decl.examples(),
                    decl.is_plugin().is_some(),
                    decl.get_block_id().is_some(),
                )
            })
            .collect()
    }

    pub fn get_block(&self, block_id: BlockId) -> &Block {
        self.blocks
            .get(block_id)
            .expect("internal error: missing block")
    }

    pub fn get_module(&self, module_id: ModuleId) -> &Module {
        self.modules
            .get(module_id)
            .expect("internal error: missing module")
    }

    pub fn next_span_start(&self) -> usize {
        if let Some((_, _, last)) = self.file_contents.last() {
            *last
        } else {
            0
        }
    }

    pub fn files(&self) -> impl Iterator<Item = &(String, usize, usize)> {
        self.files.iter()
    }

    pub fn get_filename(&self, file_id: usize) -> String {
        for file in self.files.iter().enumerate() {
            if file.0 == file_id {
                return file.1 .0.clone();
            }
        }

        "<unknown>".into()
    }

    pub fn get_file_source(&self, file_id: usize) -> String {
        for file in self.files.iter().enumerate() {
            if file.0 == file_id {
                let contents = self.get_span_contents(&Span {
                    start: file.1 .1,
                    end: file.1 .2,
                });
                let output = String::from_utf8_lossy(contents).to_string();

                return output;
            }
        }

        "<unknown>".into()
    }

    pub fn add_file(&mut self, filename: String, contents: Vec<u8>) -> usize {
        let next_span_start = self.next_span_start();
        let next_span_end = next_span_start + contents.len();

        self.file_contents
            .push((contents, next_span_start, next_span_end));

        self.files.push((filename, next_span_start, next_span_end));

        self.num_files() - 1
    }

    pub fn get_cwd(&self) -> Option<String> {
        if let Some(pwd_value) = self.get_env_var(PWD_ENV) {
            pwd_value.as_string().ok()
        } else {
            None
        }
    }

    #[cfg(not(windows))]
    pub fn get_sig_quit(&self) -> &Option<Arc<AtomicBool>> {
        &self.sig_quit
    }

    #[cfg(windows)]
    pub fn get_sig_quit(&self) -> &Option<Arc<AtomicBool>> {
        &None
    }

    #[cfg(not(windows))]
    pub fn set_sig_quit(&mut self, sig_quit: Arc<AtomicBool>) {
        self.sig_quit = Some(sig_quit)
    }

    pub fn set_config_path(&mut self, key: &str, val: PathBuf) {
        self.config_path.insert(key.to_string(), val);
    }

    pub fn get_config_path(&self, key: &str) -> Option<&PathBuf> {
        self.config_path.get(key)
    }
}

/// A temporary extension to the global state. This handles bridging between the global state and the
/// additional declarations and scope changes that are not yet part of the global scope.
///
/// This working set is created by the parser as a way of handling declarations and scope changes that
/// may later be merged or dropped (and not merged) depending on the needs of the code calling the parser.
pub struct StateWorkingSet<'a> {
    pub permanent_state: &'a EngineState,
    pub delta: StateDelta,
    pub external_commands: Vec<Vec<u8>>,
    pub type_scope: TypeScope,
    /// Current working directory relative to the file being parsed right now
    pub currently_parsed_cwd: Option<PathBuf>,
    /// All previously parsed module files. Used to protect against circular imports.
    pub parsed_module_files: Vec<PathBuf>,
}

/// A temporary placeholder for expression types. It is used to keep track of the input types
/// for each expression in a pipeline
pub struct TypeScope {
    /// Layers that map the type inputs that are found in each parsed block
    outputs: Vec<Vec<Type>>,
    /// The last know output from a parsed block
    last_output: Type,
}

impl Default for TypeScope {
    fn default() -> Self {
        Self {
            outputs: Vec::new(),
            last_output: Type::Any,
        }
    }
}

impl TypeScope {
    pub fn get_previous(&self) -> &Type {
        match self.outputs.last().and_then(|v| v.last()) {
            Some(input) => input,
            None => &Type::Any,
        }
    }

    pub fn get_last_output(&self) -> Type {
        self.last_output.clone()
    }

    pub fn add_type(&mut self, input: Type) {
        match self.outputs.last_mut() {
            Some(v) => v.push(input),
            None => self.outputs.push(vec![input]),
        }
    }

    pub fn enter_scope(&mut self) {
        self.outputs.push(Vec::new())
    }

    pub fn exit_scope(&mut self) -> Option<Vec<Type>> {
        self.last_output = self.get_previous().clone();
        self.outputs.pop()
    }
}

/// A delta (or change set) between the current global state and a possible future global state. Deltas
/// can be applied to the global state to update it to contain both previous state and the state held
/// within the delta.
pub struct StateDelta {
    files: Vec<(String, usize, usize)>,
    pub(crate) file_contents: Vec<(Vec<u8>, usize, usize)>,
    vars: Vec<Variable>,          // indexed by VarId
    decls: Vec<Box<dyn Command>>, // indexed by DeclId
    aliases: Vec<Vec<Span>>,      // indexed by AliasId
    pub blocks: Vec<Block>,       // indexed by BlockId
    modules: Vec<Module>,         // indexed by ModuleId
    pub scope: Vec<ScopeFrame>,
    #[cfg(feature = "plugin")]
    plugins_changed: bool, // marks whether plugin file should be updated
}

impl StateDelta {
    pub fn new(engine_state: &EngineState) -> Self {
        let last_overlay = engine_state.last_overlay(&[]);
        let scope_frame = ScopeFrame::with_empty_overlay(
            engine_state.last_overlay_name(&[]).to_owned(),
            last_overlay.origin,
            last_overlay.prefixed,
        );

        StateDelta {
            files: vec![],
            file_contents: vec![],
            vars: vec![],
            decls: vec![],
            aliases: vec![],
            blocks: vec![],
            modules: vec![],
            scope: vec![scope_frame],
            #[cfg(feature = "plugin")]
            plugins_changed: false,
        }
    }

    pub fn num_files(&self) -> usize {
        self.files.len()
    }

    pub fn num_decls(&self) -> usize {
        self.decls.len()
    }

    pub fn num_aliases(&self) -> usize {
        self.aliases.len()
    }

    pub fn num_blocks(&self) -> usize {
        self.blocks.len()
    }

    pub fn num_modules(&self) -> usize {
        self.modules.len()
    }

    pub fn last_scope_frame_mut(&mut self) -> &mut ScopeFrame {
        self.scope
            .last_mut()
            .expect("internal error: missing required scope frame")
    }

    pub fn last_scope_frame(&self) -> &ScopeFrame {
        self.scope
            .last()
            .expect("internal error: missing required scope frame")
    }

    pub fn last_overlay_mut(&mut self) -> Option<&mut OverlayFrame> {
        let last_scope = self
            .scope
            .last_mut()
            .expect("internal error: missing required scope frame");

        if let Some(last_overlay_id) = last_scope.active_overlays.last() {
            Some(
                &mut last_scope
                    .overlays
                    .get_mut(*last_overlay_id)
                    .expect("internal error: missing required overlay")
                    .1,
            )
        } else {
            None
        }
    }

    pub fn last_overlay(&self) -> Option<&OverlayFrame> {
        let last_scope = self
            .scope
            .last()
            .expect("internal error: missing required scope frame");

        if let Some(last_overlay_id) = last_scope.active_overlays.last() {
            Some(
                &last_scope
                    .overlays
                    .get(*last_overlay_id)
                    .expect("internal error: missing required overlay")
                    .1,
            )
        } else {
            None
        }
    }

    pub fn enter_scope(&mut self) {
        self.scope.push(ScopeFrame::new());
    }

    pub fn exit_scope(&mut self) {
        self.scope.pop();
    }
}

impl<'a> StateWorkingSet<'a> {
    pub fn new(permanent_state: &'a EngineState) -> Self {
        Self {
            delta: StateDelta::new(permanent_state),
            permanent_state,
            external_commands: vec![],
            type_scope: TypeScope::default(),
            currently_parsed_cwd: None,
            parsed_module_files: vec![],
        }
    }

    pub fn num_files(&self) -> usize {
        self.delta.num_files() + self.permanent_state.num_files()
    }

    pub fn num_decls(&self) -> usize {
        self.delta.num_decls() + self.permanent_state.num_decls()
    }

    pub fn num_aliases(&self) -> usize {
        self.delta.num_aliases() + self.permanent_state.num_aliases()
    }

    pub fn num_blocks(&self) -> usize {
        self.delta.num_blocks() + self.permanent_state.num_blocks()
    }

    pub fn num_modules(&self) -> usize {
        self.delta.num_modules() + self.permanent_state.num_modules()
    }

    pub fn unique_overlay_names(&self) -> HashSet<&Vec<u8>> {
        let mut names: HashSet<&Vec<u8>> = self
            .permanent_state
            .active_overlay_names(&[])
            .into_iter()
            .collect();

        for scope_frame in self.delta.scope.iter().rev() {
            for overlay_id in scope_frame.active_overlays.iter().rev() {
                let (overlay_name, _) = scope_frame
                    .overlays
                    .get(*overlay_id)
                    .expect("internal error: missing overlay");

                names.insert(overlay_name);
                names.retain(|n| !scope_frame.removed_overlays.contains(n));
            }
        }

        names
    }

    pub fn num_overlays(&self) -> usize {
        self.unique_overlay_names().len()
    }

    pub fn add_decl(&mut self, decl: Box<dyn Command>) -> DeclId {
        let name = decl.name().as_bytes().to_vec();
        let input_type = decl.signature().input_type;

        self.delta.decls.push(decl);
        let decl_id = self.num_decls() - 1;

        self.last_overlay_mut()
            .insert_decl(name, input_type, decl_id);

        decl_id
    }

    pub fn use_decls(&mut self, decls: Vec<(Vec<u8>, DeclId)>) {
        let overlay_frame = self.last_overlay_mut();

        for (name, decl_id) in decls {
            overlay_frame.insert_decl(name, Type::Any, decl_id);
            overlay_frame.visibility.use_decl_id(&decl_id);
        }
    }

    pub fn use_aliases(&mut self, aliases: Vec<(Vec<u8>, AliasId)>) {
        let overlay_frame = self.last_overlay_mut();

        for (name, alias_id) in aliases {
            overlay_frame.aliases.insert(name, alias_id);
            overlay_frame.visibility.use_alias_id(&alias_id);
        }
    }

    pub fn add_predecl(&mut self, decl: Box<dyn Command>) -> Option<DeclId> {
        let name = decl.name().as_bytes().to_vec();

        self.delta.decls.push(decl);
        let decl_id = self.num_decls() - 1;

        self.delta
            .last_scope_frame_mut()
            .predecls
            .insert(name, decl_id)
    }

    #[cfg(feature = "plugin")]
    pub fn mark_plugins_file_dirty(&mut self) {
        self.delta.plugins_changed = true;
    }

    pub fn merge_predecl(&mut self, name: &[u8]) -> Option<DeclId> {
        self.move_predecls_to_overlay();

        let overlay_frame = self.last_overlay_mut();

        if let Some(decl_id) = overlay_frame.predecls.remove(name) {
            overlay_frame.insert_decl(name.into(), Type::Any, decl_id);

            return Some(decl_id);
        }

        None
    }

    pub fn move_predecls_to_overlay(&mut self) {
        let predecls: HashMap<Vec<u8>, DeclId> =
            self.delta.last_scope_frame_mut().predecls.drain().collect();

        self.last_overlay_mut().predecls.extend(predecls);
    }

    pub fn hide_decl(&mut self, name: &[u8]) -> Option<DeclId> {
        let mut removed_overlays = vec![];
        let mut visibility: Visibility = Visibility::new();

        // Since we can mutate scope frames in delta, remove the id directly
        for scope_frame in self.delta.scope.iter_mut().rev() {
            for overlay_id in scope_frame
                .active_overlay_ids(&mut removed_overlays)
                .iter_mut()
                .rev()
            {
                let overlay_frame = scope_frame.get_overlay_mut(*overlay_id);

                visibility.append(&overlay_frame.visibility);

                if let Some(decl_id) = overlay_frame.get_decl(name, &Type::Any) {
                    if visibility.is_decl_id_visible(&decl_id) {
                        // Hide decl only if it's not already hidden
                        overlay_frame.visibility.hide_decl_id(&decl_id);
                        return Some(decl_id);
                    }
                }
            }
        }

        // We cannot mutate the permanent state => store the information in the current overlay frame
        // for scope in self.permanent_state.scope.iter().rev() {
        for overlay_frame in self
            .permanent_state
            .active_overlays(&removed_overlays)
            .iter()
            .rev()
        {
            visibility.append(&overlay_frame.visibility);

            if let Some(decl_id) = overlay_frame.get_decl(name, &Type::Any) {
                if visibility.is_decl_id_visible(&decl_id) {
                    // Hide decl only if it's not already hidden
                    self.last_overlay_mut().visibility.hide_decl_id(&decl_id);
                    return Some(decl_id);
                }
            }
        }

        None
    }

    pub fn use_alias(&mut self, alias_id: &AliasId) {
        let mut removed_overlays = vec![];
        let mut visibility: Visibility = Visibility::new();

        // Since we can mutate scope frames in delta, remove the id directly
        for scope_frame in self.delta.scope.iter_mut().rev() {
            for overlay_id in scope_frame
                .active_overlay_ids(&mut removed_overlays)
                .iter()
                .rev()
            {
                let overlay_frame = scope_frame.get_overlay_mut(*overlay_id);

                visibility.append(&overlay_frame.visibility);

                if !visibility.is_alias_id_visible(alias_id) {
                    // Use alias only if it's already hidden
                    overlay_frame.visibility.use_alias_id(alias_id);

                    return;
                }
            }
        }

        // We cannot mutate the permanent state => store the information in the current scope frame
        // for scope in self.permanent_state.scope.iter().rev() {
        for overlay_frame in self
            .permanent_state
            .active_overlays(&removed_overlays)
            .iter()
            .rev()
        {
            visibility.append(&overlay_frame.visibility);

            if !visibility.is_alias_id_visible(alias_id) {
                // Hide alias only if it's not already hidden
                self.last_overlay_mut().visibility.use_alias_id(alias_id);

                return;
            }
        }
    }

    pub fn hide_alias(&mut self, name: &[u8]) -> Option<AliasId> {
        let mut removed_overlays = vec![];
        let mut visibility: Visibility = Visibility::new();

        // Since we can mutate scope frames in delta, remove the id directly
        for scope_frame in self.delta.scope.iter_mut().rev() {
            for overlay_id in scope_frame
                .active_overlay_ids(&mut removed_overlays)
                .iter()
                .rev()
            {
                let overlay_frame = scope_frame.get_overlay_mut(*overlay_id);

                visibility.append(&overlay_frame.visibility);

                if let Some(alias_id) = overlay_frame.aliases.get(name) {
                    if visibility.is_alias_id_visible(alias_id) {
                        // Hide alias only if it's not already hidden
                        overlay_frame.visibility.hide_alias_id(alias_id);
                        return Some(*alias_id);
                    }
                }
            }
        }

        // We cannot mutate the permanent state => store the information in the current scope frame
        // for scope in self.permanent_state.scope.iter().rev() {
        for overlay_frame in self
            .permanent_state
            .active_overlays(&removed_overlays)
            .iter()
            .rev()
        {
            visibility.append(&overlay_frame.visibility);

            if let Some(alias_id) = overlay_frame.aliases.get(name) {
                if visibility.is_alias_id_visible(alias_id) {
                    // Hide alias only if it's not already hidden
                    self.last_overlay_mut().visibility.hide_alias_id(alias_id);

                    return Some(*alias_id);
                }
            }
        }

        None
    }

    pub fn hide_decls(&mut self, decls: &[Vec<u8>]) {
        for decl in decls.iter() {
            self.hide_decl(decl); // let's assume no errors
        }
    }

    pub fn hide_aliases(&mut self, aliases: &[Vec<u8>]) {
        for alias in aliases.iter() {
            self.hide_alias(alias); // let's assume no errors
        }
    }

    pub fn add_block(&mut self, block: Block) -> BlockId {
        self.delta.blocks.push(block);

        self.num_blocks() - 1
    }

    pub fn add_module(&mut self, name: &str, module: Module) -> ModuleId {
        let name = name.as_bytes().to_vec();

        self.delta.modules.push(module);
        let module_id = self.num_modules() - 1;

        self.last_overlay_mut().modules.insert(name, module_id);

        module_id
    }

    pub fn next_span_start(&self) -> usize {
        let permanent_span_start = self.permanent_state.next_span_start();

        if let Some((_, _, last)) = self.delta.file_contents.last() {
            *last
        } else {
            permanent_span_start
        }
    }

    pub fn global_span_offset(&self) -> usize {
        self.permanent_state.next_span_start()
    }

    pub fn files(&'a self) -> impl Iterator<Item = &(String, usize, usize)> {
        self.permanent_state.files().chain(self.delta.files.iter())
    }

    pub fn get_filename(&self, file_id: usize) -> String {
        for file in self.files().enumerate() {
            if file.0 == file_id {
                return file.1 .0.clone();
            }
        }

        "<unknown>".into()
    }

    pub fn get_file_source(&self, file_id: usize) -> String {
        for file in self.files().enumerate() {
            if file.0 == file_id {
                let output = String::from_utf8_lossy(self.get_span_contents(Span {
                    start: file.1 .1,
                    end: file.1 .2,
                }))
                .to_string();

                return output;
            }
        }

        "<unknown>".into()
    }

    pub fn add_file(&mut self, filename: String, contents: &[u8]) -> usize {
        let next_span_start = self.next_span_start();
        let next_span_end = next_span_start + contents.len();

        self.delta
            .file_contents
            .push((contents.to_vec(), next_span_start, next_span_end));

        self.delta
            .files
            .push((filename, next_span_start, next_span_end));

        self.num_files() - 1
    }

    pub fn get_span_contents(&self, span: Span) -> &[u8] {
        let permanent_end = self.permanent_state.next_span_start();
        if permanent_end <= span.start {
            for (contents, start, finish) in &self.delta.file_contents {
                if (span.start >= *start) && (span.end <= *finish) {
                    let begin = span.start - start;
                    let mut end = span.end - start;
                    if begin > end {
                        end = *finish - permanent_end;
                    }

                    return &contents[begin..end];
                }
            }
        } else {
            return self.permanent_state.get_span_contents(&span);
        }

        panic!("internal error: missing span contents in file cache")
    }

    pub fn enter_scope(&mut self) {
        self.delta.enter_scope();
    }

    pub fn exit_scope(&mut self) {
        self.delta.exit_scope();
    }

    pub fn find_predecl(&self, name: &[u8]) -> Option<DeclId> {
        let mut removed_overlays = vec![];

        for scope_frame in self.delta.scope.iter().rev() {
            if let Some(decl_id) = scope_frame.predecls.get(name) {
                return Some(*decl_id);
            }

            for overlay_frame in scope_frame
                .active_overlays(&mut removed_overlays)
                .iter()
                .rev()
            {
                if let Some(decl_id) = overlay_frame.predecls.get(name) {
                    return Some(*decl_id);
                }
            }
        }

        None
    }

    pub fn find_decl(&self, name: &[u8], input: &Type) -> Option<DeclId> {
        let mut removed_overlays = vec![];

        let mut visibility: Visibility = Visibility::new();

        for scope_frame in self.delta.scope.iter().rev() {
            if let Some(decl_id) = scope_frame.predecls.get(name) {
                if visibility.is_decl_id_visible(decl_id) {
                    return Some(*decl_id);
                }
            }

            // check overlay in delta
            for overlay_frame in scope_frame
                .active_overlays(&mut removed_overlays)
                .iter()
                .rev()
            {
                visibility.append(&overlay_frame.visibility);

                if let Some(decl_id) = overlay_frame.predecls.get(name) {
                    if visibility.is_decl_id_visible(decl_id) {
                        return Some(*decl_id);
                    }
                }

                if let Some(decl_id) = overlay_frame.get_decl(name, input) {
                    if visibility.is_decl_id_visible(&decl_id) {
                        return Some(decl_id);
                    }
                }
            }
        }

        // check overlay in perma
        for overlay_frame in self
            .permanent_state
            .active_overlays(&removed_overlays)
            .iter()
            .rev()
        {
            visibility.append(&overlay_frame.visibility);

            if let Some(decl_id) = overlay_frame.get_decl(name, input) {
                if visibility.is_decl_id_visible(&decl_id) {
                    return Some(decl_id);
                }
            }
        }

        None
    }

    pub fn find_alias(&self, name: &[u8]) -> Option<AliasId> {
        let mut removed_overlays = vec![];
        let mut visibility: Visibility = Visibility::new();

        for scope_frame in self.delta.scope.iter().rev() {
            for overlay_frame in scope_frame
                .active_overlays(&mut removed_overlays)
                .iter()
                .rev()
            {
                visibility.append(&overlay_frame.visibility);

                if let Some(alias_id) = overlay_frame.aliases.get(name) {
                    if visibility.is_alias_id_visible(alias_id) {
                        return Some(*alias_id);
                    }
                }
            }
        }

        for overlay_frame in self
            .permanent_state
            .active_overlays(&removed_overlays)
            .iter()
            .rev()
        {
            visibility.append(&overlay_frame.visibility);

            if let Some(alias_id) = overlay_frame.aliases.get(name) {
                if visibility.is_alias_id_visible(alias_id) {
                    return Some(*alias_id);
                }
            }
        }

        None
    }

    pub fn find_module(&self, name: &[u8]) -> Option<ModuleId> {
        let mut removed_overlays = vec![];

        for scope_frame in self.delta.scope.iter().rev() {
            for overlay_frame in scope_frame
                .active_overlays(&mut removed_overlays)
                .iter()
                .rev()
            {
                if let Some(module_id) = overlay_frame.modules.get(name) {
                    return Some(*module_id);
                }
            }
        }

        for overlay_frame in self
            .permanent_state
            .active_overlays(&removed_overlays)
            .iter()
            .rev()
        {
            if let Some(module_id) = overlay_frame.modules.get(name) {
                return Some(*module_id);
            }
        }

        None
    }

    pub fn contains_decl_partial_match(&self, name: &[u8]) -> bool {
        let mut removed_overlays = vec![];

        for scope_frame in self.delta.scope.iter().rev() {
            for overlay_frame in scope_frame
                .active_overlays(&mut removed_overlays)
                .iter()
                .rev()
            {
                for decl in &overlay_frame.decls {
                    if decl.0 .0.starts_with(name) {
                        return true;
                    }
                }
            }
        }

        for overlay_frame in self
            .permanent_state
            .active_overlays(&removed_overlays)
            .iter()
            .rev()
        {
            for decl in &overlay_frame.decls {
                if decl.0 .0.starts_with(name) {
                    return true;
                }
            }
        }

        false
    }

    pub fn next_var_id(&self) -> VarId {
        let num_permanent_vars = self.permanent_state.num_vars();
        num_permanent_vars + self.delta.vars.len()
    }

    pub fn find_variable(&self, name: &[u8]) -> Option<VarId> {
        let mut removed_overlays = vec![];

        for scope_frame in self.delta.scope.iter().rev() {
            for overlay_frame in scope_frame
                .active_overlays(&mut removed_overlays)
                .iter()
                .rev()
            {
                if let Some(var_id) = overlay_frame.vars.get(name) {
                    return Some(*var_id);
                }
            }
        }

        for overlay_frame in self
            .permanent_state
            .active_overlays(&removed_overlays)
            .iter()
            .rev()
        {
            if let Some(var_id) = overlay_frame.vars.get(name) {
                return Some(*var_id);
            }
        }

        None
    }

    pub fn add_variable(
        &mut self,
        mut name: Vec<u8>,
        span: Span,
        ty: Type,
        mutable: bool,
    ) -> VarId {
        let next_id = self.next_var_id();

        // correct name if necessary
        if !name.starts_with(b"$") {
            name.insert(0, b'$');
        }

        self.last_overlay_mut().vars.insert(name, next_id);

        self.delta.vars.push(Variable::new(span, ty, mutable));

        next_id
    }

    pub fn add_alias(&mut self, name: Vec<u8>, replacement: Vec<Span>) {
        self.delta.aliases.push(replacement);
        let alias_id = self.num_aliases() - 1;

        let last = self.last_overlay_mut();

        last.aliases.insert(name, alias_id);
        last.visibility.use_alias_id(&alias_id);
    }

    pub fn get_cwd(&self) -> String {
        let pwd = self
            .permanent_state
            .get_env_var(PWD_ENV)
            .expect("internal error: can't find PWD");
        pwd.as_string().expect("internal error: PWD not a string")
    }

    pub fn get_env_var(&self, name: &str) -> Option<&Value> {
        self.permanent_state.get_env_var(name)
    }

    pub fn get_config(&self) -> &Config {
        &self.permanent_state.config
    }

    pub fn list_env(&self) -> Vec<String> {
        let mut env_vars = vec![];

        for env_var in self.permanent_state.env_vars.clone().into_iter() {
            env_vars.push(env_var.0)
        }

        env_vars
    }

    pub fn set_variable_type(&mut self, var_id: VarId, ty: Type) {
        let num_permanent_vars = self.permanent_state.num_vars();
        if var_id < num_permanent_vars {
            panic!("Internal error: attempted to set into permanent state from working set")
        } else {
            self.delta.vars[var_id - num_permanent_vars].ty = ty;
        }
    }

    pub fn get_variable(&self, var_id: VarId) -> &Variable {
        let num_permanent_vars = self.permanent_state.num_vars();
        if var_id < num_permanent_vars {
            self.permanent_state.get_var(var_id)
        } else {
            self.delta
                .vars
                .get(var_id - num_permanent_vars)
                .expect("internal error: missing variable")
        }
    }

    pub fn get_variable_if_possible(&self, var_id: VarId) -> Option<&Variable> {
        let num_permanent_vars = self.permanent_state.num_vars();
        if var_id < num_permanent_vars {
            Some(self.permanent_state.get_var(var_id))
        } else {
            self.delta.vars.get(var_id - num_permanent_vars)
        }
    }

    #[allow(clippy::borrowed_box)]
    pub fn get_decl(&self, decl_id: DeclId) -> &Box<dyn Command> {
        let num_permanent_decls = self.permanent_state.num_decls();
        if decl_id < num_permanent_decls {
            self.permanent_state.get_decl(decl_id)
        } else {
            self.delta
                .decls
                .get(decl_id - num_permanent_decls)
                .expect("internal error: missing declaration")
        }
    }

    pub fn get_decl_mut(&mut self, decl_id: DeclId) -> &mut Box<dyn Command> {
        let num_permanent_decls = self.permanent_state.num_decls();
        if decl_id < num_permanent_decls {
            panic!("internal error: can only mutate declarations in working set")
        } else {
            self.delta
                .decls
                .get_mut(decl_id - num_permanent_decls)
                .expect("internal error: missing declaration")
        }
    }

    pub fn get_alias(&self, alias_id: AliasId) -> &[Span] {
        let num_permanent_aliases = self.permanent_state.num_aliases();
        if alias_id < num_permanent_aliases {
            self.permanent_state.get_alias(alias_id)
        } else {
            self.delta
                .aliases
                .get(alias_id - num_permanent_aliases)
                .expect("internal error: missing alias")
                .as_ref()
        }
    }

    pub fn find_commands_by_predicate(
        &self,
        predicate: impl Fn(&[u8]) -> bool,
    ) -> Vec<(Vec<u8>, Option<String>)> {
        let mut output = vec![];

        for scope_frame in self.delta.scope.iter().rev() {
            for overlay_id in scope_frame.active_overlays.iter().rev() {
                let overlay_frame = scope_frame.get_overlay(*overlay_id);

                for decl in &overlay_frame.decls {
                    if overlay_frame.visibility.is_decl_id_visible(decl.1) && predicate(&decl.0 .0)
                    {
                        let command = self.get_decl(*decl.1);
                        output.push((decl.0 .0.clone(), Some(command.usage().to_string())));
                    }
                }
            }
        }

        let mut permanent = self.permanent_state.find_commands_by_predicate(predicate);

        output.append(&mut permanent);

        output
    }

    pub fn find_aliases_by_predicate(
        &self,
        predicate: impl Fn(&[u8]) -> bool + Copy,
    ) -> Vec<Vec<u8>> {
        let mut output = vec![];

        for scope_frame in self.delta.scope.iter().rev() {
            for overlay_id in scope_frame.active_overlays.iter().rev() {
                let overlay_frame = scope_frame.get_overlay(*overlay_id);

                for alias in &overlay_frame.aliases {
                    if overlay_frame.visibility.is_alias_id_visible(alias.1) && predicate(alias.0) {
                        output.push(alias.0.clone());
                    }
                }
            }
        }

        let mut permanent = self.permanent_state.find_aliases_by_predicate(predicate);

        output.append(&mut permanent);

        output
    }

    pub fn get_block(&self, block_id: BlockId) -> &Block {
        let num_permanent_blocks = self.permanent_state.num_blocks();
        if block_id < num_permanent_blocks {
            self.permanent_state.get_block(block_id)
        } else {
            self.delta
                .blocks
                .get(block_id - num_permanent_blocks)
                .expect("internal error: missing block")
        }
    }

    pub fn get_module(&self, module_id: ModuleId) -> &Module {
        let num_permanent_modules = self.permanent_state.num_modules();
        if module_id < num_permanent_modules {
            self.permanent_state.get_module(module_id)
        } else {
            self.delta
                .modules
                .get(module_id - num_permanent_modules)
                .expect("internal error: missing module")
        }
    }

    pub fn get_block_mut(&mut self, block_id: BlockId) -> &mut Block {
        let num_permanent_blocks = self.permanent_state.num_blocks();
        if block_id < num_permanent_blocks {
            panic!("Attempt to mutate a block that is in the permanent (immutable) state")
        } else {
            self.delta
                .blocks
                .get_mut(block_id - num_permanent_blocks)
                .expect("internal error: missing block")
        }
    }

    pub fn has_overlay(&self, name: &[u8]) -> bool {
        for scope_frame in self.delta.scope.iter().rev() {
            if scope_frame
                .overlays
                .iter()
                .any(|(overlay_name, _)| name == overlay_name)
            {
                return true;
            }
        }

        self.permanent_state.has_overlay(name)
    }

    pub fn find_overlay(&self, name: &[u8]) -> Option<&OverlayFrame> {
        for scope_frame in self.delta.scope.iter().rev() {
            if let Some(overlay_id) = scope_frame.find_overlay(name) {
                return Some(scope_frame.get_overlay(overlay_id));
            }
        }

        self.permanent_state
            .find_overlay(name)
            .map(|id| self.permanent_state.get_overlay(id))
    }

    pub fn last_overlay_name(&self) -> &Vec<u8> {
        let mut removed_overlays = vec![];

        for scope_frame in self.delta.scope.iter().rev() {
            if let Some(last_name) = scope_frame
                .active_overlay_names(&mut removed_overlays)
                .iter()
                .rev()
                .last()
            {
                return last_name;
            }
        }

        self.permanent_state.last_overlay_name(&removed_overlays)
    }

    pub fn last_overlay(&self) -> &OverlayFrame {
        let mut removed_overlays = vec![];

        for scope_frame in self.delta.scope.iter().rev() {
            if let Some(last_overlay) = scope_frame
                .active_overlays(&mut removed_overlays)
                .iter()
                .rev()
                .last()
            {
                return last_overlay;
            }
        }

        self.permanent_state.last_overlay(&removed_overlays)
    }

    pub fn last_overlay_mut(&mut self) -> &mut OverlayFrame {
        if self.delta.last_overlay_mut().is_none() {
            // If there is no overlay, automatically activate the last one
            let overlay_frame = self.last_overlay();
            let name = self.last_overlay_name().to_vec();
            let origin = overlay_frame.origin;
            let prefixed = overlay_frame.prefixed;
            self.add_overlay(name, origin, vec![], vec![], prefixed);
        }

        self.delta
            .last_overlay_mut()
            .expect("internal error: missing added overlay")
    }

    /// Collect all decls that belong to an overlay
    pub fn decls_of_overlay(&self, name: &[u8]) -> HashMap<Vec<u8>, DeclId> {
        let mut result = HashMap::new();

        if let Some(overlay_id) = self.permanent_state.find_overlay(name) {
            let overlay_frame = self.permanent_state.get_overlay(overlay_id);

            for (decl_key, decl_id) in &overlay_frame.decls {
                result.insert(decl_key.0.to_owned(), *decl_id);
            }
        }

        for scope_frame in self.delta.scope.iter() {
            if let Some(overlay_id) = scope_frame.find_overlay(name) {
                let overlay_frame = scope_frame.get_overlay(overlay_id);

                for (decl_key, decl_id) in &overlay_frame.decls {
                    result.insert(decl_key.0.to_owned(), *decl_id);
                }
            }
        }

        result
    }

    /// Collect all aliases that belong to an overlay
    pub fn aliases_of_overlay(&self, name: &[u8]) -> HashMap<Vec<u8>, DeclId> {
        let mut result = HashMap::new();

        if let Some(overlay_id) = self.permanent_state.find_overlay(name) {
            let overlay_frame = self.permanent_state.get_overlay(overlay_id);

            for (alias_name, alias_id) in &overlay_frame.aliases {
                result.insert(alias_name.to_owned(), *alias_id);
            }
        }

        for scope_frame in self.delta.scope.iter() {
            if let Some(overlay_id) = scope_frame.find_overlay(name) {
                let overlay_frame = scope_frame.get_overlay(overlay_id);

                for (alias_name, alias_id) in &overlay_frame.aliases {
                    result.insert(alias_name.to_owned(), *alias_id);
                }
            }
        }

        result
    }

    pub fn add_overlay(
        &mut self,
        name: Vec<u8>,
        origin: ModuleId,
        decls: Vec<(Vec<u8>, DeclId)>,
        aliases: Vec<(Vec<u8>, AliasId)>,
        prefixed: bool,
    ) {
        let last_scope_frame = self.delta.last_scope_frame_mut();

        last_scope_frame
            .removed_overlays
            .retain(|removed_name| removed_name != &name);

        let overlay_id = if let Some(overlay_id) = last_scope_frame.find_overlay(&name) {
            last_scope_frame.get_overlay_mut(overlay_id).origin = origin;

            overlay_id
        } else {
            last_scope_frame
                .overlays
                .push((name, OverlayFrame::from_origin(origin, prefixed)));
            last_scope_frame.overlays.len() - 1
        };

        last_scope_frame
            .active_overlays
            .retain(|id| id != &overlay_id);
        last_scope_frame.active_overlays.push(overlay_id);

        self.move_predecls_to_overlay();

        self.use_decls(decls);
        self.use_aliases(aliases);
    }

    pub fn remove_overlay(&mut self, name: &[u8], keep_custom: bool) {
        let last_scope_frame = self.delta.last_scope_frame_mut();

        let maybe_module_id = if let Some(overlay_id) = last_scope_frame.find_overlay(name) {
            last_scope_frame
                .active_overlays
                .retain(|id| id != &overlay_id);

            Some(last_scope_frame.get_overlay(overlay_id).origin)
        } else {
            self.permanent_state
                .find_overlay(name)
                .map(|id| self.permanent_state.get_overlay(id).origin)
        };

        if let Some(module_id) = maybe_module_id {
            last_scope_frame.removed_overlays.push(name.to_owned());

            if keep_custom {
                let origin_module = self.get_module(module_id);

                let decls = self
                    .decls_of_overlay(name)
                    .into_iter()
                    .filter(|(n, _)| !origin_module.has_decl(n))
                    .collect();

                let aliases = self
                    .aliases_of_overlay(name)
                    .into_iter()
                    .filter(|(n, _)| !origin_module.has_alias(n))
                    .collect();

                self.use_decls(decls);
                self.use_aliases(aliases);
            }
        }
    }

    pub fn render(self) -> StateDelta {
        self.delta
    }
}

impl Default for Visibility {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ScopeFrame {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for EngineState {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> miette::SourceCode for &StateWorkingSet<'a> {
    fn read_span<'b>(
        &'b self,
        span: &miette::SourceSpan,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> Result<Box<dyn miette::SpanContents + 'b>, miette::MietteError> {
        let debugging = std::env::var("MIETTE_DEBUG").is_ok();
        if debugging {
            let finding_span = "Finding span in StateWorkingSet";
            dbg!(finding_span, span);
        }
        for (filename, start, end) in self.files() {
            if debugging {
                dbg!(&filename, start, end);
            }
            if span.offset() >= *start && span.offset() + span.len() <= *end {
                if debugging {
                    let found_file = "Found matching file";
                    dbg!(found_file);
                }
                let our_span = Span {
                    start: *start,
                    end: *end,
                };
                // We need to move to a local span because we're only reading
                // the specific file contents via self.get_span_contents.
                let local_span = (span.offset() - *start, span.len()).into();
                if debugging {
                    dbg!(&local_span);
                }
                let span_contents = self.get_span_contents(our_span);
                if debugging {
                    dbg!(String::from_utf8_lossy(span_contents));
                }
                let span_contents = span_contents.read_span(
                    &local_span,
                    context_lines_before,
                    context_lines_after,
                )?;
                let content_span = span_contents.span();
                // Back to "global" indexing
                let retranslated = (content_span.offset() + start, content_span.len()).into();
                if debugging {
                    dbg!(&retranslated);
                }

                let data = span_contents.data();
                if filename == "<cli>" {
                    if debugging {
                        let success_cli = "Successfully read CLI span";
                        dbg!(success_cli, String::from_utf8_lossy(data));
                    }
                    return Ok(Box::new(miette::MietteSpanContents::new(
                        data,
                        retranslated,
                        span_contents.line(),
                        span_contents.column(),
                        span_contents.line_count(),
                    )));
                } else {
                    if debugging {
                        let success_file = "Successfully read file span";
                        dbg!(success_file);
                    }
                    return Ok(Box::new(miette::MietteSpanContents::new_named(
                        filename.clone(),
                        data,
                        retranslated,
                        span_contents.line(),
                        span_contents.column(),
                        span_contents.line_count(),
                    )));
                }
            }
        }
        Err(miette::MietteError::OutOfBounds)
    }
}

#[cfg(test)]
mod engine_state_tests {
    use super::*;

    #[test]
    fn add_file_gives_id() {
        let engine_state = EngineState::new();
        let mut engine_state = StateWorkingSet::new(&engine_state);
        let id = engine_state.add_file("test.nu".into(), &[]);

        assert_eq!(id, 0);
    }

    #[test]
    fn add_file_gives_id_including_parent() {
        let mut engine_state = EngineState::new();
        let parent_id = engine_state.add_file("test.nu".into(), vec![]);

        let mut working_set = StateWorkingSet::new(&engine_state);
        let working_set_id = working_set.add_file("child.nu".into(), &[]);

        assert_eq!(parent_id, 0);
        assert_eq!(working_set_id, 1);
    }

    #[test]
    fn merge_states() -> Result<(), ShellError> {
        let mut engine_state = EngineState::new();
        engine_state.add_file("test.nu".into(), vec![]);

        let delta = {
            let mut working_set = StateWorkingSet::new(&engine_state);
            working_set.add_file("child.nu".into(), &[]);
            working_set.render()
        };

        engine_state.merge_delta(delta)?;

        assert_eq!(engine_state.num_files(), 2);
        assert_eq!(&engine_state.files[0].0, "test.nu");
        assert_eq!(&engine_state.files[1].0, "child.nu");

        Ok(())
    }
}
