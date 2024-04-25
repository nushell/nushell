use crate::{
    ast::Block,
    debugger::{Debugger, NoopDebugger},
    engine::{
        usage::{build_usage, Usage},
        CachedFile, Command, CommandType, EnvVars, OverlayFrame, ScopeFrame, Stack, StateDelta,
        Variable, Visibility, DEFAULT_OVERLAY_NAME,
    },
    BlockId, Category, Config, DeclId, Example, FileId, HistoryConfig, Module, ModuleId, OverlayId,
    ShellError, Signature, Span, Type, Value, VarId, VirtualPathId,
};
use fancy_regex::Regex;
use lru::LruCache;
use std::{
    collections::HashMap,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex, MutexGuard, PoisonError,
    },
};

type PoisonDebuggerError<'a> = PoisonError<MutexGuard<'a, Box<dyn Debugger>>>;

#[cfg(feature = "plugin")]
use crate::{PluginRegistryFile, PluginRegistryItem, RegisteredPlugin};

pub static PWD_ENV: &str = "PWD";

#[derive(Clone, Debug)]
pub enum VirtualPath {
    File(FileId),
    Dir(Vec<VirtualPathId>),
}

pub struct ReplState {
    pub buffer: String,
    // A byte position, as `EditCommand::MoveToPosition` is also a byte position
    pub cursor_pos: usize,
}

pub struct IsDebugging(AtomicBool);

impl IsDebugging {
    pub fn new(val: bool) -> Self {
        IsDebugging(AtomicBool::new(val))
    }
}

impl Clone for IsDebugging {
    fn clone(&self) -> Self {
        IsDebugging(AtomicBool::new(self.0.load(Ordering::Relaxed)))
    }
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
/// Many of the larger objects in this structure are stored within `Arc` to decrease the cost of
/// cloning `EngineState`. While `Arc`s are generally immutable, they can be modified using
/// `Arc::make_mut`, which automatically clones to a new allocation if there are other copies of
/// the `Arc` already in use, but will let us modify the `Arc` directly if we have the only
/// reference to it.
///
/// Note that the runtime stack is not part of this global state. Runtime stacks are handled differently,
/// but they also rely on using IDs rather than full definitions.
#[derive(Clone)]
pub struct EngineState {
    files: Vec<CachedFile>,
    pub(super) virtual_paths: Vec<(String, VirtualPath)>,
    vars: Vec<Variable>,
    decls: Arc<Vec<Box<dyn Command + 'static>>>,
    // The Vec is wrapped in Arc so that if we don't need to modify the list, we can just clone
    // the reference and not have to clone each individual Arc inside. These lists can be
    // especially long, so it helps
    pub(super) blocks: Arc<Vec<Arc<Block>>>,
    pub(super) modules: Arc<Vec<Arc<Module>>>,
    usage: Usage,
    pub scope: ScopeFrame,
    pub ctrlc: Option<Arc<AtomicBool>>,
    pub env_vars: Arc<EnvVars>,
    pub previous_env_vars: Arc<HashMap<String, Value>>,
    pub config: Arc<Config>,
    pub pipeline_externals_state: Arc<(AtomicU32, AtomicU32)>,
    pub repl_state: Arc<Mutex<ReplState>>,
    pub table_decl_id: Option<usize>,
    #[cfg(feature = "plugin")]
    pub plugin_path: Option<PathBuf>,
    #[cfg(feature = "plugin")]
    plugins: Vec<Arc<dyn RegisteredPlugin>>,
    config_path: HashMap<String, PathBuf>,
    pub history_enabled: bool,
    pub history_session_id: i64,
    // Path to the file Nushell is currently evaluating, or None if we're in an interactive session.
    pub file: Option<PathBuf>,
    pub regex_cache: Arc<Mutex<LruCache<String, Regex>>>,
    pub is_interactive: bool,
    pub is_login: bool,
    startup_time: i64,
    is_debugging: IsDebugging,
    pub debugger: Arc<Mutex<Box<dyn Debugger>>>,
}

// The max number of compiled regexes to keep around in a LRU cache, arbitrarily chosen
const REGEX_CACHE_SIZE: usize = 100; // must be nonzero, otherwise will panic

pub const NU_VARIABLE_ID: usize = 0;
pub const IN_VARIABLE_ID: usize = 1;
pub const ENV_VARIABLE_ID: usize = 2;
// NOTE: If you add more to this list, make sure to update the > checks based on the last in the list

impl EngineState {
    pub fn new() -> Self {
        Self {
            files: vec![],
            virtual_paths: vec![],
            vars: vec![
                Variable::new(Span::new(0, 0), Type::Any, false),
                Variable::new(Span::new(0, 0), Type::Any, false),
                Variable::new(Span::new(0, 0), Type::Any, false),
                Variable::new(Span::new(0, 0), Type::Any, false),
                Variable::new(Span::new(0, 0), Type::Any, false),
            ],
            decls: Arc::new(vec![]),
            blocks: Arc::new(vec![]),
            modules: Arc::new(vec![Arc::new(Module::new(
                DEFAULT_OVERLAY_NAME.as_bytes().to_vec(),
            ))]),
            usage: Usage::new(),
            // make sure we have some default overlay:
            scope: ScopeFrame::with_empty_overlay(
                DEFAULT_OVERLAY_NAME.as_bytes().to_vec(),
                0,
                false,
            ),
            ctrlc: None,
            env_vars: Arc::new(
                [(DEFAULT_OVERLAY_NAME.to_string(), HashMap::new())]
                    .into_iter()
                    .collect(),
            ),
            previous_env_vars: Arc::new(HashMap::new()),
            config: Arc::new(Config::default()),
            pipeline_externals_state: Arc::new((AtomicU32::new(0), AtomicU32::new(0))),
            repl_state: Arc::new(Mutex::new(ReplState {
                buffer: "".to_string(),
                cursor_pos: 0,
            })),
            table_decl_id: None,
            #[cfg(feature = "plugin")]
            plugin_path: None,
            #[cfg(feature = "plugin")]
            plugins: vec![],
            config_path: HashMap::new(),
            history_enabled: true,
            history_session_id: 0,
            file: None,
            regex_cache: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(REGEX_CACHE_SIZE).expect("tried to create cache of size zero"),
            ))),
            is_interactive: false,
            is_login: false,
            startup_time: -1,
            is_debugging: IsDebugging::new(false),
            debugger: Arc::new(Mutex::new(Box::new(NoopDebugger))),
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
        self.virtual_paths.extend(delta.virtual_paths);
        self.vars.extend(delta.vars);
        self.usage.merge_with(delta.usage);

        // Avoid potentially cloning the Arcs if we aren't adding anything
        if !delta.decls.is_empty() {
            Arc::make_mut(&mut self.decls).extend(delta.decls);
        }
        if !delta.blocks.is_empty() {
            Arc::make_mut(&mut self.blocks).extend(delta.blocks);
        }
        if !delta.modules.is_empty() {
            Arc::make_mut(&mut self.modules).extend(delta.modules);
        }

        let first = delta.scope.remove(0);

        for (delta_name, delta_overlay) in first.clone().overlays {
            if let Some((_, existing_overlay)) = self
                .scope
                .overlays
                .iter_mut()
                .find(|(name, _)| name == &delta_name)
            {
                // Updating existing overlay
                for item in delta_overlay.decls.into_iter() {
                    existing_overlay.decls.insert(item.0, item.1);
                }
                for item in delta_overlay.vars.into_iter() {
                    existing_overlay.vars.insert(item.0, item.1);
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
        if !delta.plugins.is_empty() {
            // Replace plugins that overlap in identity.
            for plugin in std::mem::take(&mut delta.plugins) {
                if let Some(existing) = self
                    .plugins
                    .iter_mut()
                    .find(|p| p.identity().name() == plugin.identity().name())
                {
                    // Stop the existing plugin, so that the new plugin definitely takes over
                    existing.stop()?;
                    *existing = plugin;
                } else {
                    self.plugins.push(plugin);
                }
            }
        }

        #[cfg(feature = "plugin")]
        if !delta.plugin_registry_items.is_empty() {
            // Update the plugin file with the new signatures.
            if self.plugin_path.is_some() {
                self.update_plugin_file(std::mem::take(&mut delta.plugin_registry_items))?;
            }
        }

        Ok(())
    }

    /// Merge the environment from the runtime Stack into the engine state
    pub fn merge_env(
        &mut self,
        stack: &mut Stack,
        cwd: impl AsRef<Path>,
    ) -> Result<(), ShellError> {
        let mut config_updated = false;

        for mut scope in stack.env_vars.drain(..) {
            for (overlay_name, mut env) in scope.drain() {
                if let Some(env_vars) = Arc::make_mut(&mut self.env_vars).get_mut(&overlay_name) {
                    // Updating existing overlay
                    for (k, v) in env.drain() {
                        if k == "config" {
                            // Don't insert the record as the "config" env var as-is.
                            // Instead, mutate a clone of it with into_config(), and put THAT in env_vars.
                            let mut new_record = v.clone();
                            let (config, error) = new_record.parse_as_config(&self.config);
                            self.config = Arc::new(config);
                            config_updated = true;
                            env_vars.insert(k, new_record);
                            if let Some(e) = error {
                                return Err(e);
                            }
                        } else {
                            env_vars.insert(k, v);
                        }
                    }
                } else {
                    // Pushing a new overlay
                    Arc::make_mut(&mut self.env_vars).insert(overlay_name, env);
                }
            }
        }

        // TODO: better error
        std::env::set_current_dir(cwd)?;

        if config_updated {
            // Make plugin GC config changes take effect immediately.
            #[cfg(feature = "plugin")]
            self.update_plugin_gc_configs(&self.config.plugin_gc);
        }

        Ok(())
    }

    pub fn has_overlay(&self, name: &[u8]) -> bool {
        self.scope
            .overlays
            .iter()
            .any(|(overlay_name, _)| name == overlay_name)
    }

    pub fn active_overlay_ids<'a, 'b>(
        &'b self,
        removed_overlays: &'a [Vec<u8>],
    ) -> impl DoubleEndedIterator<Item = &OverlayId> + 'a
    where
        'b: 'a,
    {
        self.scope.active_overlays.iter().filter(|id| {
            !removed_overlays
                .iter()
                .any(|name| name == self.get_overlay_name(**id))
        })
    }

    pub fn active_overlays<'a, 'b>(
        &'b self,
        removed_overlays: &'a [Vec<u8>],
    ) -> impl DoubleEndedIterator<Item = &OverlayFrame> + 'a
    where
        'b: 'a,
    {
        self.active_overlay_ids(removed_overlays)
            .map(|id| self.get_overlay(*id))
    }

    pub fn active_overlay_names<'a, 'b>(
        &'b self,
        removed_overlays: &'a [Vec<u8>],
    ) -> impl DoubleEndedIterator<Item = &[u8]> + 'a
    where
        'b: 'a,
    {
        self.active_overlay_ids(removed_overlays)
            .map(|id| self.get_overlay_name(*id))
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

    pub fn last_overlay_name(&self, removed_overlays: &[Vec<u8>]) -> &[u8] {
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

    pub fn get_overlay_name(&self, overlay_id: OverlayId) -> &[u8] {
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

        if let Some(env_vars) = Arc::make_mut(&mut self.env_vars).get_mut(&overlay_name) {
            env_vars.insert(name, val);
        } else {
            Arc::make_mut(&mut self.env_vars)
                .insert(overlay_name, [(name, val)].into_iter().collect());
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

    // Get the path environment variable in a platform agnostic way
    pub fn get_path_env_var(&self) -> Option<&Value> {
        let env_path_name_windows: &str = "Path";
        let env_path_name_nix: &str = "PATH";

        for overlay_id in self.scope.active_overlays.iter().rev() {
            let overlay_name = String::from_utf8_lossy(self.get_overlay_name(*overlay_id));
            if let Some(env_vars) = self.env_vars.get(overlay_name.as_ref()) {
                if let Some(val) = env_vars.get(env_path_name_nix) {
                    return Some(val);
                } else if let Some(val) = env_vars.get(env_path_name_windows) {
                    return Some(val);
                } else {
                    return None;
                }
            }
        }

        None
    }

    #[cfg(feature = "plugin")]
    pub fn plugins(&self) -> &[Arc<dyn RegisteredPlugin>] {
        &self.plugins
    }

    #[cfg(feature = "plugin")]
    pub fn update_plugin_file(
        &self,
        updated_items: Vec<PluginRegistryItem>,
    ) -> Result<(), ShellError> {
        // Updating the signatures plugin file with the added signatures
        use std::fs::File;

        let plugin_path = self
            .plugin_path
            .as_ref()
            .ok_or_else(|| ShellError::GenericError {
                error: "Plugin file path not set".into(),
                msg: "".into(),
                span: None,
                help: Some("you may be running nu with --no-config-file".into()),
                inner: vec![],
            })?;

        // Read the current contents of the plugin file if it exists
        let mut contents = match File::open(plugin_path.as_path()) {
            Ok(mut plugin_file) => PluginRegistryFile::read_from(&mut plugin_file, None),
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    Ok(PluginRegistryFile::default())
                } else {
                    Err(ShellError::GenericError {
                        error: "Failed to open plugin file".into(),
                        msg: "".into(),
                        span: None,
                        help: None,
                        inner: vec![err.into()],
                    })
                }
            }
        }?;

        // Update the given signatures
        for item in updated_items {
            contents.upsert_plugin(item);
        }

        // Write it to the same path
        let plugin_file =
            File::create(plugin_path.as_path()).map_err(|err| ShellError::GenericError {
                error: "Failed to write plugin file".into(),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![err.into()],
            })?;

        contents.write_to(plugin_file, None)
    }

    /// Update plugins with new garbage collection config
    #[cfg(feature = "plugin")]
    fn update_plugin_gc_configs(&self, plugin_gc: &crate::PluginGcConfigs) {
        for plugin in &self.plugins {
            plugin.set_gc_config(plugin_gc.get(plugin.identity().name()));
        }
    }

    pub fn num_files(&self) -> usize {
        self.files.len()
    }

    pub fn num_virtual_paths(&self) -> usize {
        self.virtual_paths.len()
    }

    pub fn num_vars(&self) -> usize {
        self.vars.len()
    }

    pub fn num_decls(&self) -> usize {
        self.decls.len()
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
        for cached_file in self.files.iter() {
            let string = String::from_utf8_lossy(&cached_file.content);
            println!("{string}");
        }
    }

    pub fn find_decl(&self, name: &[u8], removed_overlays: &[Vec<u8>]) -> Option<DeclId> {
        let mut visibility: Visibility = Visibility::new();

        for overlay_frame in self.active_overlays(removed_overlays).rev() {
            visibility.append(&overlay_frame.visibility);

            if let Some(decl_id) = overlay_frame.get_decl(name) {
                if visibility.is_decl_id_visible(&decl_id) {
                    return Some(decl_id);
                }
            }
        }

        None
    }

    pub fn find_decl_name(&self, decl_id: DeclId, removed_overlays: &[Vec<u8>]) -> Option<&[u8]> {
        let mut visibility: Visibility = Visibility::new();

        for overlay_frame in self.active_overlays(removed_overlays).rev() {
            visibility.append(&overlay_frame.visibility);

            if visibility.is_decl_id_visible(&decl_id) {
                for (name, id) in overlay_frame.decls.iter() {
                    if id == &decl_id {
                        return Some(name);
                    }
                }
            }
        }

        None
    }

    pub fn get_module_comments(&self, module_id: ModuleId) -> Option<&[Span]> {
        self.usage.get_module_comments(module_id)
    }

    #[cfg(feature = "plugin")]
    pub fn plugin_decls(&self) -> impl Iterator<Item = &Box<dyn Command + 'static>> {
        let mut unique_plugin_decls = HashMap::new();

        // Make sure there are no duplicate decls: Newer one overwrites the older one
        for decl in self.decls.iter().filter(|d| d.is_plugin()) {
            unique_plugin_decls.insert(decl.name(), decl);
        }

        let mut plugin_decls: Vec<(&str, &Box<dyn Command>)> =
            unique_plugin_decls.into_iter().collect();

        // Sort the plugins by name so we don't end up with a random plugin file each time
        plugin_decls.sort_by(|a, b| a.0.cmp(b.0));
        plugin_decls.into_iter().map(|(_, decl)| decl)
    }

    pub fn find_module(&self, name: &[u8], removed_overlays: &[Vec<u8>]) -> Option<ModuleId> {
        for overlay_frame in self.active_overlays(removed_overlays).rev() {
            if let Some(module_id) = overlay_frame.modules.get(name) {
                return Some(*module_id);
            }
        }

        None
    }

    pub fn which_module_has_decl(
        &self,
        decl_name: &[u8],
        removed_overlays: &[Vec<u8>],
    ) -> Option<&[u8]> {
        for overlay_frame in self.active_overlays(removed_overlays).rev() {
            for (module_name, module_id) in overlay_frame.modules.iter() {
                let module = self.get_module(*module_id);
                if module.has_decl(decl_name) {
                    return Some(module_name);
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
        ignore_deprecated: bool,
    ) -> Vec<(Vec<u8>, Option<String>, CommandType)> {
        let mut output = vec![];

        for overlay_frame in self.active_overlays(&[]).rev() {
            for decl in &overlay_frame.decls {
                if overlay_frame.visibility.is_decl_id_visible(decl.1) && predicate(decl.0) {
                    let command = self.get_decl(*decl.1);
                    if ignore_deprecated && command.signature().category == Category::Removed {
                        continue;
                    }
                    output.push((
                        decl.0.clone(),
                        Some(command.usage().to_string()),
                        command.command_type(),
                    ));
                }
            }
        }

        output
    }

    pub fn get_span_contents(&self, span: Span) -> &[u8] {
        for file in &self.files {
            if file.covered_span.contains_span(span) {
                return &file.content
                    [(span.start - file.covered_span.start)..(span.end - file.covered_span.start)];
            }
        }
        &[0u8; 0]
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }

    pub fn set_config(&mut self, conf: Config) {
        #[cfg(feature = "plugin")]
        if conf.plugin_gc != self.config.plugin_gc {
            // Make plugin GC config changes take effect immediately.
            self.update_plugin_gc_configs(&conf.plugin_gc);
        }

        self.config = Arc::new(conf);
    }

    /// Fetch the configuration for a plugin
    ///
    /// The `plugin` must match the registered name of a plugin.  For `register nu_plugin_example`
    /// the plugin name to use will be `"example"`
    pub fn get_plugin_config(&self, plugin: &str) -> Option<&Value> {
        self.config.plugins.get(plugin)
    }

    /// Returns the configuration settings for command history or `None` if history is disabled
    pub fn history_config(&self) -> Option<HistoryConfig> {
        self.history_enabled.then(|| self.config.history)
    }

    pub fn get_var(&self, var_id: VarId) -> &Variable {
        self.vars
            .get(var_id)
            .expect("internal error: missing variable")
    }

    pub fn get_constant(&self, var_id: VarId) -> Option<&Value> {
        let var = self.get_var(var_id);
        var.const_val.as_ref()
    }

    pub fn set_variable_const_val(&mut self, var_id: VarId, val: Value) {
        self.vars[var_id].const_val = Some(val);
    }

    pub fn get_decl(&self, decl_id: DeclId) -> &dyn Command {
        self.decls
            .get(decl_id)
            .expect("internal error: missing declaration")
            .as_ref()
    }

    /// Get all commands within scope, sorted by the commands' names
    pub fn get_decls_sorted(
        &self,
        include_hidden: bool,
    ) -> impl Iterator<Item = (Vec<u8>, DeclId)> {
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

        let mut decls: Vec<(Vec<u8>, DeclId)> = decls_map.into_iter().collect();

        decls.sort_by(|a, b| a.0.cmp(&b.0));
        decls.into_iter()
    }

    pub fn get_signature(&self, decl: &dyn Command) -> Signature {
        if let Some(block_id) = decl.get_block_id() {
            *self.blocks[block_id].signature.clone()
        } else {
            decl.signature()
        }
    }

    /// Get signatures of all commands within scope.
    pub fn get_signatures(&self, include_hidden: bool) -> Vec<Signature> {
        self.get_decls_sorted(include_hidden)
            .map(|(_, id)| {
                let decl = self.get_decl(id);

                self.get_signature(decl).update_from_command(decl)
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
    ) -> Vec<(Signature, Vec<Example>, bool, bool, bool)> {
        self.get_decls_sorted(include_hidden)
            .map(|(_, id)| {
                let decl = self.get_decl(id);

                let signature = self.get_signature(decl).update_from_command(decl);

                (
                    signature,
                    decl.examples(),
                    decl.is_plugin(),
                    decl.get_block_id().is_some(),
                    decl.is_parser_keyword(),
                )
            })
            .collect()
    }

    pub fn get_block(&self, block_id: BlockId) -> &Arc<Block> {
        self.blocks
            .get(block_id)
            .expect("internal error: missing block")
    }

    /// Optionally get a block by id, if it exists
    ///
    /// Prefer to use [`.get_block()`] in most cases - `BlockId`s that don't exist are normally a
    /// compiler error. This only exists to stop plugins from crashing the engine if they send us
    /// something invalid.
    pub fn try_get_block(&self, block_id: BlockId) -> Option<&Arc<Block>> {
        self.blocks.get(block_id)
    }

    pub fn get_module(&self, module_id: ModuleId) -> &Module {
        self.modules
            .get(module_id)
            .expect("internal error: missing module")
    }

    pub fn get_virtual_path(&self, virtual_path_id: VirtualPathId) -> &(String, VirtualPath) {
        self.virtual_paths
            .get(virtual_path_id)
            .expect("internal error: missing virtual path")
    }

    pub fn next_span_start(&self) -> usize {
        if let Some(cached_file) = self.files.last() {
            cached_file.covered_span.end
        } else {
            0
        }
    }

    pub fn files(&self) -> impl Iterator<Item = &CachedFile> {
        self.files.iter()
    }

    pub fn add_file(&mut self, filename: Arc<str>, content: Arc<[u8]>) -> FileId {
        let next_span_start = self.next_span_start();
        let next_span_end = next_span_start + content.len();

        let covered_span = Span::new(next_span_start, next_span_end);

        self.files.push(CachedFile {
            name: filename,
            content,
            covered_span,
        });

        self.num_files() - 1
    }

    pub fn get_cwd(&self) -> Option<String> {
        if let Some(pwd_value) = self.get_env_var(PWD_ENV) {
            pwd_value.coerce_string().ok()
        } else {
            None
        }
    }

    pub fn set_config_path(&mut self, key: &str, val: PathBuf) {
        self.config_path.insert(key.to_string(), val);
    }

    pub fn get_config_path(&self, key: &str) -> Option<&PathBuf> {
        self.config_path.get(key)
    }

    pub fn build_usage(&self, spans: &[Span]) -> (String, String) {
        let comment_lines: Vec<&[u8]> = spans
            .iter()
            .map(|span| self.get_span_contents(*span))
            .collect();
        build_usage(&comment_lines)
    }

    pub fn build_module_usage(&self, module_id: ModuleId) -> Option<(String, String)> {
        self.get_module_comments(module_id)
            .map(|comment_spans| self.build_usage(comment_spans))
    }

    pub fn current_work_dir(&self) -> String {
        self.get_env_var("PWD")
            .map(|d| d.coerce_string().unwrap_or_default())
            .unwrap_or_default()
    }

    // TODO: see if we can completely get rid of this
    pub fn get_file_contents(&self) -> &[CachedFile] {
        &self.files
    }

    pub fn get_startup_time(&self) -> i64 {
        self.startup_time
    }

    pub fn set_startup_time(&mut self, startup_time: i64) {
        self.startup_time = startup_time;
    }

    pub fn activate_debugger(
        &self,
        debugger: Box<dyn Debugger>,
    ) -> Result<(), PoisonDebuggerError> {
        let mut locked_debugger = self.debugger.lock()?;
        *locked_debugger = debugger;
        locked_debugger.activate();
        self.is_debugging.0.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub fn deactivate_debugger(&self) -> Result<Box<dyn Debugger>, PoisonDebuggerError> {
        let mut locked_debugger = self.debugger.lock()?;
        locked_debugger.deactivate();
        let ret = std::mem::replace(&mut *locked_debugger, Box::new(NoopDebugger));
        self.is_debugging.0.store(false, Ordering::Relaxed);
        Ok(ret)
    }

    pub fn is_debugging(&self) -> bool {
        self.is_debugging.0.load(Ordering::Relaxed)
    }

    pub fn recover_from_panic(&mut self) {
        if Mutex::is_poisoned(&self.repl_state) {
            self.repl_state = Arc::new(Mutex::new(ReplState {
                buffer: "".to_string(),
                cursor_pos: 0,
            }));
        }
        if Mutex::is_poisoned(&self.regex_cache) {
            self.regex_cache = Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(REGEX_CACHE_SIZE).expect("tried to create cache of size zero"),
            )));
        }
    }
}

impl Default for EngineState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod engine_state_tests {
    use crate::engine::StateWorkingSet;
    use std::str::{from_utf8, Utf8Error};

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
        let parent_id = engine_state.add_file("test.nu".into(), Arc::new([]));

        let mut working_set = StateWorkingSet::new(&engine_state);
        let working_set_id = working_set.add_file("child.nu".into(), &[]);

        assert_eq!(parent_id, 0);
        assert_eq!(working_set_id, 1);
    }

    #[test]
    fn merge_states() -> Result<(), ShellError> {
        let mut engine_state = EngineState::new();
        engine_state.add_file("test.nu".into(), Arc::new([]));

        let delta = {
            let mut working_set = StateWorkingSet::new(&engine_state);
            let _ = working_set.add_file("child.nu".into(), &[]);
            working_set.render()
        };

        engine_state.merge_delta(delta)?;

        assert_eq!(engine_state.num_files(), 2);
        assert_eq!(&*engine_state.files[0].name, "test.nu");
        assert_eq!(&*engine_state.files[1].name, "child.nu");

        Ok(())
    }

    #[test]
    fn list_variables() -> Result<(), Utf8Error> {
        let varname = "something";
        let varname_with_sigil = "$".to_owned() + varname;
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);
        working_set.add_variable(
            varname.as_bytes().into(),
            Span { start: 0, end: 1 },
            Type::Int,
            false,
        );
        let variables = working_set
            .list_variables()
            .into_iter()
            .map(from_utf8)
            .collect::<Result<Vec<&str>, Utf8Error>>()?;
        assert_eq!(variables, vec![varname_with_sigil]);
        Ok(())
    }

    #[test]
    fn get_plugin_config() {
        let mut engine_state = EngineState::new();

        assert!(
            engine_state.get_plugin_config("example").is_none(),
            "Unexpected plugin configuration"
        );

        let mut plugins = HashMap::new();
        plugins.insert("example".into(), Value::string("value", Span::test_data()));

        let mut config = engine_state.get_config().clone();
        config.plugins = plugins;

        engine_state.set_config(config);

        assert!(
            engine_state.get_plugin_config("example").is_some(),
            "Plugin configuration not found"
        );
    }
}
