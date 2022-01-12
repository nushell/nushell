use super::{Command, Stack};
use crate::{
    ast::Block, BlockId, DeclId, Example, Overlay, OverlayId, ShellError, Signature, Span, Type,
    VarId,
};
use core::panic;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
};

use crate::Value;

use std::path::Path;

#[cfg(feature = "plugin")]
use std::path::PathBuf;

// Tells whether a decl etc. is visible or not
#[derive(Debug, Clone)]
struct Visibility {
    decl_ids: HashMap<DeclId, bool>,
}

impl Visibility {
    fn new() -> Self {
        Visibility {
            decl_ids: HashMap::new(),
        }
    }

    fn is_decl_id_visible(&self, decl_id: &DeclId) -> bool {
        *self.decl_ids.get(decl_id).unwrap_or(&true) // by default it's visible
    }

    fn hide_decl_id(&mut self, decl_id: &DeclId) {
        self.decl_ids.insert(*decl_id, false);
    }

    fn use_decl_id(&mut self, decl_id: &DeclId) {
        self.decl_ids.insert(*decl_id, true);
    }

    fn merge_with(&mut self, other: Visibility) {
        // overwrite own values with the other
        self.decl_ids.extend(other.decl_ids);
        // self.env_var_ids.extend(other.env_var_ids);
    }

    fn append(&mut self, other: &Visibility) {
        // take new values from other but keep own values
        for (decl_id, visible) in other.decl_ids.iter() {
            if !self.decl_ids.contains_key(decl_id) {
                self.decl_ids.insert(*decl_id, *visible);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScopeFrame {
    pub vars: HashMap<Vec<u8>, VarId>,
    predecls: HashMap<Vec<u8>, DeclId>, // temporary storage for predeclarations
    pub decls: HashMap<Vec<u8>, DeclId>,
    pub aliases: HashMap<Vec<u8>, Vec<Span>>,
    pub env_vars: HashMap<Vec<u8>, BlockId>,
    pub overlays: HashMap<Vec<u8>, OverlayId>,
    visibility: Visibility,
}

impl ScopeFrame {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            predecls: HashMap::new(),
            decls: HashMap::new(),
            aliases: HashMap::new(),
            env_vars: HashMap::new(),
            overlays: HashMap::new(),
            visibility: Visibility::new(),
        }
    }

    pub fn get_var(&self, var_name: &[u8]) -> Option<&VarId> {
        self.vars.get(var_name)
    }
}

impl Default for ScopeFrame {
    fn default() -> Self {
        Self::new()
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
    files: im::Vector<(String, usize, usize)>,
    file_contents: im::Vector<(Vec<u8>, usize, usize)>,
    vars: im::Vector<Type>,
    decls: im::Vector<Box<dyn Command + 'static>>,
    blocks: im::Vector<Block>,
    overlays: im::Vector<Overlay>,
    pub scope: im::Vector<ScopeFrame>,
    pub ctrlc: Option<Arc<AtomicBool>>,
    pub env_vars: im::HashMap<String, Value>,
    #[cfg(feature = "plugin")]
    pub plugin_signatures: Option<PathBuf>,
}

pub const NU_VARIABLE_ID: usize = 0;
pub const SCOPE_VARIABLE_ID: usize = 1;
pub const IN_VARIABLE_ID: usize = 2;
pub const CONFIG_VARIABLE_ID: usize = 3;
pub const ENV_VARIABLE_ID: usize = 4;
// NOTE: If you add more to this list, make sure to update the > checks based on the last in the list

impl EngineState {
    pub fn new() -> Self {
        Self {
            files: im::vector![],
            file_contents: im::vector![],
            vars: im::vector![
                Type::Unknown,
                Type::Unknown,
                Type::Unknown,
                Type::Unknown,
                Type::Unknown
            ],
            decls: im::vector![],
            blocks: im::vector![],
            overlays: im::vector![],
            scope: im::vector![ScopeFrame::new()],
            ctrlc: None,
            env_vars: im::HashMap::new(),
            #[cfg(feature = "plugin")]
            plugin_signatures: None,
        }
    }

    /// Merges a `StateDelta` onto the current state. These deltas come from a system, like the parser, that
    /// creates a new set of definitions and visible symbols in the current scope. We make this transactional
    /// as there are times when we want to run the parser and immediately throw away the results (namely:
    /// syntax highlighting and completions).
    ///
    /// When we want to preserve what the parser has created, we can take its output (the `StateDelta`) and
    /// use this function to merge it into the global state.
    pub fn merge_delta(
        &mut self,
        mut delta: StateDelta,
        stack: Option<&mut Stack>,
        cwd: impl AsRef<Path>,
    ) -> Result<(), ShellError> {
        // Take the mutable reference and extend the permanent state from the working set
        self.files.extend(delta.files);
        self.file_contents.extend(delta.file_contents);
        self.decls.extend(delta.decls);
        self.vars.extend(delta.vars);
        self.blocks.extend(delta.blocks);
        self.overlays.extend(delta.overlays);

        if let Some(last) = self.scope.back_mut() {
            let first = delta.scope.remove(0);
            for item in first.decls.into_iter() {
                last.decls.insert(item.0, item.1);
            }
            for item in first.vars.into_iter() {
                last.vars.insert(item.0, item.1);
            }
            for item in first.aliases.into_iter() {
                last.aliases.insert(item.0, item.1);
            }
            for item in first.overlays.into_iter() {
                last.overlays.insert(item.0, item.1);
            }
            last.visibility.merge_with(first.visibility);

            #[cfg(feature = "plugin")]
            if delta.plugins_changed {
                let result = self.update_plugin_file();

                if result.is_ok() {
                    delta.plugins_changed = false;
                }

                return result;
            }
        }

        if let Some(stack) = stack {
            for mut env_scope in stack.env_vars.drain(..) {
                for (k, v) in env_scope.drain() {
                    self.env_vars.insert(k, v);
                }
            }
        }

        // FIXME: permanent state changes like this hopefully in time can be removed
        // and be replaced by just passing the cwd in where needed
        std::env::set_current_dir(cwd)?;

        Ok(())
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
                    let (path, encoding, shell) =
                        decl.is_plugin().expect("plugin should have file name");
                    let file_name = path
                        .to_str()
                        .expect("path was checked during registration as a str");

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

                            // Each signature is stored in the plugin file with the required
                            // encoding, shell and signature
                            // This information will be used when loading the plugin
                            // information when nushell starts
                            format!(
                                "register {} -e {} {} {}\n\n",
                                file_name, encoding, shell_str, signature
                            )
                        })
                        .map_err(|err| ShellError::PluginFailedToLoad(err.to_string()))
                        .and_then(|line| {
                            plugin_file
                                .write_all(line.as_bytes())
                                .map_err(|err| ShellError::PluginFailedToLoad(err.to_string()))
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

    pub fn num_blocks(&self) -> usize {
        self.blocks.len()
    }

    pub fn num_overlays(&self) -> usize {
        self.overlays.len()
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

    pub fn find_decl(&self, name: &[u8]) -> Option<DeclId> {
        let mut visibility: Visibility = Visibility::new();

        for scope in self.scope.iter().rev() {
            visibility.append(&scope.visibility);

            if let Some(decl_id) = scope.decls.get(name) {
                if visibility.is_decl_id_visible(decl_id) {
                    return Some(*decl_id);
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

    pub fn find_overlay(&self, name: &[u8]) -> Option<OverlayId> {
        for scope in self.scope.iter().rev() {
            if let Some(overlay_id) = scope.overlays.get(name) {
                return Some(*overlay_id);
            }
        }

        None
    }

    pub fn find_commands_by_prefix(&self, name: &[u8]) -> Vec<Vec<u8>> {
        let mut output = vec![];

        for scope in self.scope.iter().rev() {
            for decl in &scope.decls {
                if decl.0.starts_with(name) {
                    output.push(decl.0.clone());
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

        panic!("internal error: span missing in file contents cache")
    }

    pub fn get_var(&self, var_id: VarId) -> &Type {
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

    /// Get all IDs of all commands within scope, sorted by the commads' names
    pub fn get_decl_ids_sorted(&self, include_hidden: bool) -> impl Iterator<Item = DeclId> {
        let mut decls_map = HashMap::new();

        for frame in &self.scope {
            let frame_decls = if include_hidden {
                frame.decls.clone()
            } else {
                frame
                    .decls
                    .clone()
                    .into_iter()
                    .filter(|(_, id)| frame.visibility.is_decl_id_visible(id))
                    .collect()
            };

            decls_map.extend(frame_decls);
        }

        let mut decls: Vec<(Vec<u8>, DeclId)> = decls_map.into_iter().collect();

        decls.sort_by(|a, b| a.0.cmp(&b.0));
        decls.into_iter().map(|(_, id)| id)
    }

    /// Get signatures of all commands within scope.
    pub fn get_signatures(&self, include_hidden: bool) -> Vec<Signature> {
        self.get_decl_ids_sorted(include_hidden)
            .map(|id| {
                let decl = self.get_decl(id);

                let mut signature = (*decl).signature();
                signature.usage = decl.usage().to_string();
                signature.extra_usage = decl.extra_usage().to_string();

                signature
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

                let mut signature = (*decl).signature();
                signature.usage = decl.usage().to_string();
                signature.extra_usage = decl.extra_usage().to_string();

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

    pub fn get_overlay(&self, overlay_id: OverlayId) -> &Overlay {
        self.overlays
            .get(overlay_id)
            .expect("internal error: missing overlay")
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
            .push_back((contents, next_span_start, next_span_end));

        self.files
            .push_back((filename, next_span_start, next_span_end));

        self.num_files() - 1
    }
}

impl Default for EngineState {
    fn default() -> Self {
        Self::new()
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
}

/// A delta (or change set) between the current global state and a possible future global state. Deltas
/// can be applied to the global state to update it to contain both previous state and the state held
/// within the delta.
pub struct StateDelta {
    files: Vec<(String, usize, usize)>,
    pub(crate) file_contents: Vec<(Vec<u8>, usize, usize)>,
    vars: Vec<Type>,              // indexed by VarId
    decls: Vec<Box<dyn Command>>, // indexed by DeclId
    blocks: Vec<Block>,           // indexed by BlockId
    overlays: Vec<Overlay>,       // indexed by OverlayId
    pub scope: Vec<ScopeFrame>,
    #[cfg(feature = "plugin")]
    plugins_changed: bool, // marks whether plugin file should be updated
}

impl StateDelta {
    pub fn num_files(&self) -> usize {
        self.files.len()
    }

    pub fn num_decls(&self) -> usize {
        self.decls.len()
    }

    pub fn num_blocks(&self) -> usize {
        self.blocks.len()
    }

    pub fn num_overlays(&self) -> usize {
        self.overlays.len()
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
            delta: StateDelta {
                files: vec![],
                file_contents: vec![],
                vars: vec![],
                decls: vec![],
                blocks: vec![],
                overlays: vec![],
                scope: vec![ScopeFrame::new()],
                #[cfg(feature = "plugin")]
                plugins_changed: false,
            },
            permanent_state,
        }
    }

    pub fn num_files(&self) -> usize {
        self.delta.num_files() + self.permanent_state.num_files()
    }

    pub fn num_decls(&self) -> usize {
        self.delta.num_decls() + self.permanent_state.num_decls()
    }

    pub fn num_blocks(&self) -> usize {
        self.delta.num_blocks() + self.permanent_state.num_blocks()
    }

    pub fn num_overlays(&self) -> usize {
        self.delta.num_overlays() + self.permanent_state.num_overlays()
    }

    pub fn add_decl(&mut self, decl: Box<dyn Command>) -> DeclId {
        let name = decl.name().as_bytes().to_vec();

        self.delta.decls.push(decl);
        let decl_id = self.num_decls() - 1;

        let scope_frame = self
            .delta
            .scope
            .last_mut()
            .expect("internal error: missing required scope frame");

        scope_frame.decls.insert(name, decl_id);
        scope_frame.visibility.use_decl_id(&decl_id);

        decl_id
    }

    pub fn use_decls(&mut self, decls: Vec<(Vec<u8>, DeclId)>) {
        let scope_frame = self
            .delta
            .scope
            .last_mut()
            .expect("internal error: missing required scope frame");

        for (name, decl_id) in decls {
            scope_frame.decls.insert(name, decl_id);
            scope_frame.visibility.use_decl_id(&decl_id);
        }
    }

    pub fn add_predecl(&mut self, decl: Box<dyn Command>) -> Option<DeclId> {
        let name = decl.name().as_bytes().to_vec();

        self.delta.decls.push(decl);
        let decl_id = self.num_decls() - 1;

        let scope_frame = self
            .delta
            .scope
            .last_mut()
            .expect("internal error: missing required scope frame");

        scope_frame.predecls.insert(name, decl_id)
    }

    #[cfg(feature = "plugin")]
    pub fn mark_plugins_file_dirty(&mut self) {
        self.delta.plugins_changed = true;
    }

    pub fn merge_predecl(&mut self, name: &[u8]) -> Option<DeclId> {
        let scope_frame = self
            .delta
            .scope
            .last_mut()
            .expect("internal error: missing required scope frame");

        if let Some(decl_id) = scope_frame.predecls.remove(name) {
            scope_frame.decls.insert(name.into(), decl_id);
            scope_frame.visibility.use_decl_id(&decl_id);

            return Some(decl_id);
        }

        None
    }

    pub fn hide_decl(&mut self, name: &[u8]) -> Option<DeclId> {
        let mut visibility: Visibility = Visibility::new();

        // Since we can mutate scope frames in delta, remove the id directly
        for scope in self.delta.scope.iter_mut().rev() {
            visibility.append(&scope.visibility);

            if let Some(decl_id) = scope.decls.remove(name) {
                return Some(decl_id);
            }
        }

        // We cannot mutate the permanent state => store the information in the current scope frame
        let last_scope_frame = self
            .delta
            .scope
            .last_mut()
            .expect("internal error: missing required scope frame");

        for scope in self.permanent_state.scope.iter().rev() {
            visibility.append(&scope.visibility);

            if let Some(decl_id) = scope.decls.get(name) {
                if visibility.is_decl_id_visible(decl_id) {
                    // Hide decl only if it's not already hidden
                    last_scope_frame.visibility.hide_decl_id(decl_id);
                    return Some(*decl_id);
                }
            }
        }

        None
    }

    pub fn hide_decls(&mut self, decls: &[(Vec<u8>, DeclId)]) {
        for decl in decls.iter() {
            self.hide_decl(&decl.0); // let's assume no errors
        }
    }

    pub fn add_block(&mut self, block: Block) -> BlockId {
        self.delta.blocks.push(block);

        self.num_blocks() - 1
    }

    pub fn add_env_var(&mut self, name_span: Span, block: Block) -> BlockId {
        self.delta.blocks.push(block);
        let block_id = self.num_blocks() - 1;
        let name = self.get_span_contents(name_span).to_vec();

        let scope_frame = self
            .delta
            .scope
            .last_mut()
            .expect("internal error: missing required scope frame");

        scope_frame.env_vars.insert(name, block_id);

        block_id
    }

    pub fn add_overlay(&mut self, name: &str, overlay: Overlay) -> OverlayId {
        let name = name.as_bytes().to_vec();

        self.delta.overlays.push(overlay);
        let overlay_id = self.num_overlays() - 1;

        let scope_frame = self
            .delta
            .scope
            .last_mut()
            .expect("internal error: missing required scope frame");

        scope_frame.overlays.insert(name, overlay_id);

        overlay_id
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
                    return &contents[(span.start - start)..(span.end - start)];
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

    pub fn find_decl(&self, name: &[u8]) -> Option<DeclId> {
        let mut visibility: Visibility = Visibility::new();

        for scope in self.delta.scope.iter().rev() {
            visibility.append(&scope.visibility);

            if let Some(decl_id) = scope.predecls.get(name) {
                return Some(*decl_id);
            }

            if let Some(decl_id) = scope.decls.get(name) {
                return Some(*decl_id);
            }
        }

        for scope in self.permanent_state.scope.iter().rev() {
            visibility.append(&scope.visibility);

            if let Some(decl_id) = scope.decls.get(name) {
                if visibility.is_decl_id_visible(decl_id) {
                    return Some(*decl_id);
                }
            }
        }

        None
    }

    pub fn find_overlay(&self, name: &[u8]) -> Option<OverlayId> {
        for scope in self.delta.scope.iter().rev() {
            if let Some(overlay_id) = scope.overlays.get(name) {
                return Some(*overlay_id);
            }
        }

        for scope in self.permanent_state.scope.iter().rev() {
            if let Some(overlay_id) = scope.overlays.get(name) {
                return Some(*overlay_id);
            }
        }

        None
    }

    // pub fn update_decl(&mut self, decl_id: usize, block: Option<BlockId>) {
    //     let decl = self.get_decl_mut(decl_id);
    //     decl.body = block;
    // }

    pub fn contains_decl_partial_match(&self, name: &[u8]) -> bool {
        for scope in self.delta.scope.iter().rev() {
            for decl in &scope.decls {
                if decl.0.starts_with(name) {
                    return true;
                }
            }
        }

        for scope in self.permanent_state.scope.iter().rev() {
            for decl in &scope.decls {
                if decl.0.starts_with(name) {
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
        for scope in self.delta.scope.iter().rev() {
            if let Some(var_id) = scope.vars.get(name) {
                return Some(*var_id);
            }
        }

        for scope in self.permanent_state.scope.iter().rev() {
            if let Some(var_id) = scope.vars.get(name) {
                return Some(*var_id);
            }
        }

        None
    }

    pub fn find_alias(&self, name: &[u8]) -> Option<&[Span]> {
        for scope in self.delta.scope.iter().rev() {
            if let Some(spans) = scope.aliases.get(name) {
                return Some(spans);
            }
        }

        for scope in self.permanent_state.scope.iter().rev() {
            if let Some(spans) = scope.aliases.get(name) {
                return Some(spans);
            }
        }

        None
    }

    pub fn add_variable(&mut self, mut name: Vec<u8>, ty: Type) -> VarId {
        let next_id = self.next_var_id();

        // correct name if necessary
        if !name.starts_with(b"$") {
            name.insert(0, b'$');
        }

        let last = self
            .delta
            .scope
            .last_mut()
            .expect("internal error: missing stack frame");

        last.vars.insert(name, next_id);

        self.delta.vars.push(ty);

        next_id
    }

    pub fn add_alias(&mut self, name: Vec<u8>, replacement: Vec<Span>) {
        let last = self
            .delta
            .scope
            .last_mut()
            .expect("internal error: missing stack frame");

        last.aliases.insert(name, replacement);
    }

    pub fn get_cwd(&self) -> String {
        let pwd = self
            .permanent_state
            .env_vars
            .get("PWD")
            .expect("internal error: can't find PWD");
        pwd.as_string().expect("internal error: PWD not a string")
    }

    pub fn set_variable_type(&mut self, var_id: VarId, ty: Type) {
        let num_permanent_vars = self.permanent_state.num_vars();
        if var_id < num_permanent_vars {
            panic!("Internal error: attempted to set into permanent state from working set")
        } else {
            self.delta.vars[var_id - num_permanent_vars] = ty;
        }
    }

    pub fn get_variable(&self, var_id: VarId) -> &Type {
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

    pub fn find_commands_by_prefix(&self, name: &[u8]) -> Vec<Vec<u8>> {
        let mut output = vec![];

        for scope in self.delta.scope.iter().rev() {
            for decl in &scope.decls {
                if decl.0.starts_with(name) {
                    output.push(decl.0.clone());
                }
            }
        }

        let mut permanent = self.permanent_state.find_commands_by_prefix(name);

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

    pub fn get_overlay(&self, overlay_id: OverlayId) -> &Overlay {
        let num_permanent_overlays = self.permanent_state.num_overlays();
        if overlay_id < num_permanent_overlays {
            self.permanent_state.get_overlay(overlay_id)
        } else {
            self.delta
                .overlays
                .get(overlay_id - num_permanent_overlays)
                .expect("internal error: missing overlay")
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

    pub fn render(self) -> StateDelta {
        self.delta
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

        let cwd = std::env::current_dir().expect("Could not get current working directory.");
        engine_state.merge_delta(delta, None, &cwd)?;

        assert_eq!(engine_state.num_files(), 2);
        assert_eq!(&engine_state.files[0].0, "test.nu");
        assert_eq!(&engine_state.files[1].0, "child.nu");

        Ok(())
    }
}
