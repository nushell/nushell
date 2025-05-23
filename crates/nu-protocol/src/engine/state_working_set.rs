use crate::{
    BlockId, Category, CompileError, Config, DeclId, FileId, GetSpan, Module, ModuleId, OverlayId,
    ParseError, ParseWarning, ResolvedImportPattern, Signature, Span, SpanId, Type, Value, VarId,
    VirtualPathId,
    ast::Block,
    engine::{
        CachedFile, Command, CommandType, EngineState, OverlayFrame, StateDelta, Variable,
        VirtualPath, Visibility, description::build_desc,
    },
};
use core::panic;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(feature = "plugin")]
use crate::{PluginIdentity, PluginRegistryItem, RegisteredPlugin};

/// A temporary extension to the global state. This handles bridging between the global state and the
/// additional declarations and scope changes that are not yet part of the global scope.
///
/// This working set is created by the parser as a way of handling declarations and scope changes that
/// may later be merged or dropped (and not merged) depending on the needs of the code calling the parser.
pub struct StateWorkingSet<'a> {
    pub permanent_state: &'a EngineState,
    pub delta: StateDelta,
    pub files: FileStack,
    /// Whether or not predeclarations are searched when looking up a command (used with aliases)
    pub search_predecls: bool,
    pub parse_errors: Vec<ParseError>,
    pub parse_warnings: Vec<ParseWarning>,
    pub compile_errors: Vec<CompileError>,
}

impl<'a> StateWorkingSet<'a> {
    pub fn new(permanent_state: &'a EngineState) -> Self {
        // Initialize the file stack with the top-level file.
        let files = if let Some(file) = permanent_state.file.clone() {
            FileStack::with_file(file)
        } else {
            FileStack::new()
        };

        Self {
            delta: StateDelta::new(permanent_state),
            permanent_state,
            files,
            search_predecls: true,
            parse_errors: vec![],
            parse_warnings: vec![],
            compile_errors: vec![],
        }
    }

    pub fn permanent(&self) -> &EngineState {
        self.permanent_state
    }

    pub fn error(&mut self, parse_error: ParseError) {
        self.parse_errors.push(parse_error)
    }

    pub fn warning(&mut self, parse_warning: ParseWarning) {
        self.parse_warnings.push(parse_warning)
    }

    pub fn num_files(&self) -> usize {
        self.delta.num_files() + self.permanent_state.num_files()
    }

    pub fn num_virtual_paths(&self) -> usize {
        self.delta.num_virtual_paths() + self.permanent_state.num_virtual_paths()
    }

    pub fn num_vars(&self) -> usize {
        self.delta.num_vars() + self.permanent_state.num_vars()
    }

    pub fn num_decls(&self) -> usize {
        self.delta.num_decls() + self.permanent_state.num_decls()
    }

    pub fn num_blocks(&self) -> usize {
        self.delta.num_blocks() + self.permanent_state.num_blocks()
    }

    pub fn num_modules(&self) -> usize {
        self.delta.num_modules() + self.permanent_state.num_modules()
    }

    pub fn unique_overlay_names(&self) -> HashSet<&[u8]> {
        let mut names: HashSet<&[u8]> = self.permanent_state.active_overlay_names(&[]).collect();

        for scope_frame in self.delta.scope.iter().rev() {
            for overlay_id in scope_frame.active_overlays.iter().rev() {
                let (overlay_name, _) = scope_frame
                    .overlays
                    .get(overlay_id.get())
                    .expect("internal error: missing overlay");

                names.insert(overlay_name);
                names.retain(|n| !scope_frame.removed_overlays.iter().any(|m| n == m));
            }
        }

        names
    }

    pub fn num_overlays(&self) -> usize {
        self.unique_overlay_names().len()
    }

    pub fn add_decl(&mut self, decl: Box<dyn Command>) -> DeclId {
        let name = decl.name().as_bytes().to_vec();

        self.delta.decls.push(decl);
        let decl_id = self.num_decls() - 1;
        let decl_id = DeclId::new(decl_id);

        self.last_overlay_mut().insert_decl(name, decl_id);

        decl_id
    }

    pub fn use_decls(&mut self, decls: Vec<(Vec<u8>, DeclId)>) {
        let overlay_frame = self.last_overlay_mut();

        for (name, decl_id) in decls {
            overlay_frame.insert_decl(name, decl_id);
            overlay_frame.visibility.use_decl_id(&decl_id);
        }
    }

    pub fn use_modules(&mut self, modules: Vec<(Vec<u8>, ModuleId)>) {
        let overlay_frame = self.last_overlay_mut();

        for (name, module_id) in modules {
            overlay_frame.insert_module(name, module_id);
            // overlay_frame.visibility.use_module_id(&module_id);  // TODO: Add hiding modules
        }
    }

    pub fn use_variables(&mut self, variables: Vec<(Vec<u8>, VarId)>) {
        let overlay_frame = self.last_overlay_mut();

        for (mut name, var_id) in variables {
            if !name.starts_with(b"$") {
                name.insert(0, b'$');
            }
            overlay_frame.insert_variable(name, var_id);
        }
    }

    pub fn add_predecl(&mut self, decl: Box<dyn Command>) -> Option<DeclId> {
        let name = decl.name().as_bytes().to_vec();

        self.delta.decls.push(decl);
        let decl_id = self.num_decls() - 1;
        let decl_id = DeclId::new(decl_id);

        self.delta
            .last_scope_frame_mut()
            .predecls
            .insert(name, decl_id)
    }

    #[cfg(feature = "plugin")]
    pub fn find_or_create_plugin(
        &mut self,
        identity: &PluginIdentity,
        make: impl FnOnce() -> Arc<dyn RegisteredPlugin>,
    ) -> Arc<dyn RegisteredPlugin> {
        // Check in delta first, then permanent_state
        if let Some(plugin) = self
            .delta
            .plugins
            .iter()
            .chain(self.permanent_state.plugins())
            .find(|p| p.identity() == identity)
        {
            plugin.clone()
        } else {
            let plugin = make();
            self.delta.plugins.push(plugin.clone());
            plugin
        }
    }

    #[cfg(feature = "plugin")]
    pub fn update_plugin_registry(&mut self, item: PluginRegistryItem) {
        self.delta.plugin_registry_items.push(item);
    }

    pub fn merge_predecl(&mut self, name: &[u8]) -> Option<DeclId> {
        self.move_predecls_to_overlay();

        let overlay_frame = self.last_overlay_mut();

        if let Some(decl_id) = overlay_frame.predecls.remove(name) {
            overlay_frame.insert_decl(name.into(), decl_id);

            return Some(decl_id);
        }

        None
    }

    fn move_predecls_to_overlay(&mut self) {
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
                .iter()
                .rev()
            {
                let overlay_frame = scope_frame.get_overlay_mut(*overlay_id);

                visibility.append(&overlay_frame.visibility);

                if let Some(decl_id) = overlay_frame.get_decl(name) {
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
            .rev()
        {
            visibility.append(&overlay_frame.visibility);

            if let Some(decl_id) = overlay_frame.get_decl(name) {
                if visibility.is_decl_id_visible(&decl_id) {
                    // Hide decl only if it's not already hidden
                    self.last_overlay_mut().visibility.hide_decl_id(&decl_id);
                    return Some(decl_id);
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

    pub fn add_block(&mut self, block: Arc<Block>) -> BlockId {
        log::trace!(
            "block id={} added, has IR = {:?}",
            self.num_blocks(),
            block.ir_block.is_some()
        );

        self.delta.blocks.push(block);

        BlockId::new(self.num_blocks() - 1)
    }

    pub fn add_module(&mut self, name: &str, module: Module, comments: Vec<Span>) -> ModuleId {
        let name = name.as_bytes().to_vec();

        self.delta.modules.push(Arc::new(module));
        let module_id = self.num_modules() - 1;
        let module_id = ModuleId::new(module_id);

        if !comments.is_empty() {
            self.delta
                .doccomments
                .add_module_comments(module_id, comments);
        }

        self.last_overlay_mut().modules.insert(name, module_id);

        module_id
    }

    pub fn get_module_comments(&self, module_id: ModuleId) -> Option<&[Span]> {
        self.delta
            .doccomments
            .get_module_comments(module_id)
            .or_else(|| self.permanent_state.get_module_comments(module_id))
    }

    pub fn next_span_start(&self) -> usize {
        let permanent_span_start = self.permanent_state.next_span_start();

        if let Some(cached_file) = self.delta.files.last() {
            cached_file.covered_span.end
        } else {
            permanent_span_start
        }
    }

    pub fn files(&self) -> impl Iterator<Item = &CachedFile> {
        self.permanent_state.files().chain(self.delta.files.iter())
    }

    pub fn get_contents_of_file(&self, file_id: FileId) -> Option<&[u8]> {
        if let Some(cached_file) = self.permanent_state.get_file_contents().get(file_id.get()) {
            return Some(&cached_file.content);
        }
        // The index subtraction will not underflow, if we hit the permanent state first.
        // Check if you try reordering for locality
        if let Some(cached_file) = self
            .delta
            .get_file_contents()
            .get(file_id.get() - self.permanent_state.num_files())
        {
            return Some(&cached_file.content);
        }

        None
    }

    #[must_use]
    pub fn add_file(&mut self, filename: String, contents: &[u8]) -> FileId {
        // First, look for the file to see if we already have it
        for (idx, cached_file) in self.files().enumerate() {
            if *cached_file.name == filename && &*cached_file.content == contents {
                return FileId::new(idx);
            }
        }

        let next_span_start = self.next_span_start();
        let next_span_end = next_span_start + contents.len();

        let covered_span = Span::new(next_span_start, next_span_end);

        self.delta.files.push(CachedFile {
            name: filename.into(),
            content: contents.into(),
            covered_span,
        });

        FileId::new(self.num_files() - 1)
    }

    #[must_use]
    pub fn add_virtual_path(&mut self, name: String, virtual_path: VirtualPath) -> VirtualPathId {
        self.delta.virtual_paths.push((name, virtual_path));

        VirtualPathId::new(self.num_virtual_paths() - 1)
    }

    pub fn get_span_for_filename(&self, filename: &str) -> Option<Span> {
        let predicate = |file: &CachedFile| &*file.name == filename;
        // search from end to start, in case there're duplicated files with the same name
        let file_id = self
            .delta
            .files
            .iter()
            .rposition(predicate)
            .map(|idx| idx + self.permanent_state.num_files())
            .or_else(|| self.permanent_state.files().rposition(predicate))?;
        let file_id = FileId::new(file_id);

        Some(self.get_span_for_file(file_id))
    }

    /// Panics:
    /// On invalid `FileId`
    ///
    /// Use with care
    pub fn get_span_for_file(&self, file_id: FileId) -> Span {
        let result = self
            .files()
            .nth(file_id.get())
            .expect("internal error: could not find source for previously parsed file");

        result.covered_span
    }

    pub fn get_span_contents(&self, span: Span) -> &[u8] {
        let permanent_end = self.permanent_state.next_span_start();
        if permanent_end <= span.start {
            for cached_file in &self.delta.files {
                if cached_file.covered_span.contains_span(span) {
                    return &cached_file.content[span.start - cached_file.covered_span.start
                        ..span.end - cached_file.covered_span.start];
                }
            }
        }

        // if no files with span were found, fall back on permanent ones
        self.permanent_state.get_span_contents(span)
    }

    pub fn enter_scope(&mut self) {
        self.delta.enter_scope();
    }

    pub fn exit_scope(&mut self) {
        self.delta.exit_scope();
    }

    /// Find the [`DeclId`](crate::DeclId) corresponding to a predeclaration with `name`.
    pub fn find_predecl(&self, name: &[u8]) -> Option<DeclId> {
        let mut removed_overlays = vec![];

        for scope_frame in self.delta.scope.iter().rev() {
            if let Some(decl_id) = scope_frame.predecls.get(name) {
                return Some(*decl_id);
            }

            for overlay_frame in scope_frame.active_overlays(&mut removed_overlays).rev() {
                if let Some(decl_id) = overlay_frame.predecls.get(name) {
                    return Some(*decl_id);
                }
            }
        }

        None
    }

    /// Find the [`DeclId`](crate::DeclId) corresponding to a declaration with `name`.
    ///
    /// Extends [`EngineState::find_decl`] to also search for predeclarations
    /// (if [`StateWorkingSet::search_predecls`] is set), and declarations from scopes existing
    /// only in [`StateDelta`].
    pub fn find_decl(&self, name: &[u8]) -> Option<DeclId> {
        let mut removed_overlays = vec![];

        let mut visibility: Visibility = Visibility::new();

        for scope_frame in self.delta.scope.iter().rev() {
            if self.search_predecls {
                if let Some(decl_id) = scope_frame.predecls.get(name) {
                    if visibility.is_decl_id_visible(decl_id) {
                        return Some(*decl_id);
                    }
                }
            }

            // check overlay in delta
            for overlay_frame in scope_frame.active_overlays(&mut removed_overlays).rev() {
                visibility.append(&overlay_frame.visibility);

                if self.search_predecls {
                    if let Some(decl_id) = overlay_frame.predecls.get(name) {
                        if visibility.is_decl_id_visible(decl_id) {
                            return Some(*decl_id);
                        }
                    }
                }

                if let Some(decl_id) = overlay_frame.get_decl(name) {
                    if visibility.is_decl_id_visible(&decl_id) {
                        return Some(decl_id);
                    }
                }
            }
        }

        // check overlay in perma
        self.permanent_state.find_decl(name, &removed_overlays)
    }

    /// Find the name of the declaration corresponding to `decl_id`.
    ///
    /// Extends [`EngineState::find_decl_name`] to also search for predeclarations (if [`StateWorkingSet::search_predecls`] is set),
    /// and declarations from scopes existing only in [`StateDelta`].
    pub fn find_decl_name(&self, decl_id: DeclId) -> Option<&[u8]> {
        let mut removed_overlays = vec![];

        let mut visibility: Visibility = Visibility::new();

        for scope_frame in self.delta.scope.iter().rev() {
            if self.search_predecls {
                for (name, id) in scope_frame.predecls.iter() {
                    if id == &decl_id {
                        return Some(name);
                    }
                }
            }

            // check overlay in delta
            for overlay_frame in scope_frame.active_overlays(&mut removed_overlays).rev() {
                visibility.append(&overlay_frame.visibility);

                if self.search_predecls {
                    for (name, id) in overlay_frame.predecls.iter() {
                        if id == &decl_id {
                            return Some(name);
                        }
                    }
                }

                if visibility.is_decl_id_visible(&decl_id) {
                    for (name, id) in overlay_frame.decls.iter() {
                        if id == &decl_id {
                            return Some(name);
                        }
                    }
                }
            }
        }

        // check overlay in perma
        self.permanent_state
            .find_decl_name(decl_id, &removed_overlays)
    }

    /// Find the [`ModuleId`](crate::ModuleId) corresponding to `name`.
    ///
    /// Extends [`EngineState::find_module`] to also search for ,
    /// and declarations from scopes existing only in [`StateDelta`].
    pub fn find_module(&self, name: &[u8]) -> Option<ModuleId> {
        let mut removed_overlays = vec![];

        for scope_frame in self.delta.scope.iter().rev() {
            for overlay_frame in scope_frame.active_overlays(&mut removed_overlays).rev() {
                if let Some(module_id) = overlay_frame.modules.get(name) {
                    return Some(*module_id);
                }
            }
        }

        for overlay_frame in self
            .permanent_state
            .active_overlays(&removed_overlays)
            .rev()
        {
            if let Some(module_id) = overlay_frame.modules.get(name) {
                return Some(*module_id);
            }
        }

        None
    }

    pub fn next_var_id(&self) -> VarId {
        let num_permanent_vars = self.permanent_state.num_vars();
        VarId::new(num_permanent_vars + self.delta.vars.len())
    }

    pub fn list_variables(&self) -> Vec<&[u8]> {
        let mut removed_overlays = vec![];
        let mut variables = HashSet::new();
        for scope_frame in self.delta.scope.iter() {
            for overlay_frame in scope_frame.active_overlays(&mut removed_overlays) {
                variables.extend(overlay_frame.vars.keys().map(|k| &k[..]));
            }
        }

        let permanent_vars = self
            .permanent_state
            .active_overlays(&removed_overlays)
            .flat_map(|overlay_frame| overlay_frame.vars.keys().map(|k| &k[..]));

        variables.extend(permanent_vars);
        variables.into_iter().collect()
    }

    pub fn find_variable(&self, name: &[u8]) -> Option<VarId> {
        let mut name = name.to_vec();
        if !name.starts_with(b"$") {
            name.insert(0, b'$');
        }
        let mut removed_overlays = vec![];

        for scope_frame in self.delta.scope.iter().rev() {
            for overlay_frame in scope_frame.active_overlays(&mut removed_overlays).rev() {
                if let Some(var_id) = overlay_frame.vars.get(&name) {
                    return Some(*var_id);
                }
            }
        }

        for overlay_frame in self
            .permanent_state
            .active_overlays(&removed_overlays)
            .rev()
        {
            if let Some(var_id) = overlay_frame.vars.get(&name) {
                return Some(*var_id);
            }
        }

        None
    }

    pub fn find_variable_in_current_frame(&self, name: &[u8]) -> Option<VarId> {
        let mut removed_overlays = vec![];

        for scope_frame in self.delta.scope.iter().rev().take(1) {
            for overlay_frame in scope_frame.active_overlays(&mut removed_overlays).rev() {
                if let Some(var_id) = overlay_frame.vars.get(name) {
                    return Some(*var_id);
                }
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

    /// Returns the current working directory as a String, which is guaranteed to be canonicalized.
    /// Returns an empty string if $env.PWD doesn't exist, is not a String, or is not an absolute path.
    ///
    /// It does NOT consider modifications to the working directory made on a stack.
    #[deprecated(since = "0.92.3", note = "please use `EngineState::cwd()` instead")]
    pub fn get_cwd(&self) -> String {
        self.permanent_state
            .cwd(None)
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default()
    }

    pub fn get_env_var(&self, name: &str) -> Option<&Value> {
        self.permanent_state.get_env_var(name)
    }

    /// Returns a reference to the config stored at permanent state
    ///
    /// At runtime, you most likely want to call [`Stack::get_config()`][super::Stack::get_config()]
    /// because this method does not capture environment updates during runtime.
    pub fn get_config(&self) -> &Arc<Config> {
        &self.permanent_state.config
    }

    pub fn set_variable_type(&mut self, var_id: VarId, ty: Type) {
        let num_permanent_vars = self.permanent_state.num_vars();
        if var_id.get() < num_permanent_vars {
            panic!("Internal error: attempted to set into permanent state from working set")
        } else {
            self.delta.vars[var_id.get() - num_permanent_vars].ty = ty;
        }
    }

    pub fn set_variable_const_val(&mut self, var_id: VarId, val: Value) {
        let num_permanent_vars = self.permanent_state.num_vars();
        if var_id.get() < num_permanent_vars {
            panic!("Internal error: attempted to set into permanent state from working set")
        } else {
            self.delta.vars[var_id.get() - num_permanent_vars].const_val = Some(val);
        }
    }

    pub fn get_variable(&self, var_id: VarId) -> &Variable {
        let num_permanent_vars = self.permanent_state.num_vars();
        if var_id.get() < num_permanent_vars {
            self.permanent_state.get_var(var_id)
        } else {
            self.delta
                .vars
                .get(var_id.get() - num_permanent_vars)
                .expect("internal error: missing variable")
        }
    }

    pub fn get_variable_if_possible(&self, var_id: VarId) -> Option<&Variable> {
        let num_permanent_vars = self.permanent_state.num_vars();
        if var_id.get() < num_permanent_vars {
            Some(self.permanent_state.get_var(var_id))
        } else {
            self.delta.vars.get(var_id.get() - num_permanent_vars)
        }
    }

    pub fn get_constant(&self, var_id: VarId) -> Result<&Value, ParseError> {
        let var = self.get_variable(var_id);

        if let Some(const_val) = &var.const_val {
            Ok(const_val)
        } else {
            Err(ParseError::InternalError(
                "constant does not have a constant value".into(),
                var.declaration_span,
            ))
        }
    }

    pub fn get_decl(&self, decl_id: DeclId) -> &dyn Command {
        let num_permanent_decls = self.permanent_state.num_decls();
        if decl_id.get() < num_permanent_decls {
            self.permanent_state.get_decl(decl_id)
        } else {
            self.delta
                .decls
                .get(decl_id.get() - num_permanent_decls)
                .expect("internal error: missing declaration")
                .as_ref()
        }
    }

    pub fn get_decl_mut(&mut self, decl_id: DeclId) -> &mut Box<dyn Command> {
        let num_permanent_decls = self.permanent_state.num_decls();
        if decl_id.get() < num_permanent_decls {
            panic!("internal error: can only mutate declarations in working set")
        } else {
            self.delta
                .decls
                .get_mut(decl_id.get() - num_permanent_decls)
                .expect("internal error: missing declaration")
        }
    }

    pub fn get_signature(&self, decl: &dyn Command) -> Signature {
        if let Some(block_id) = decl.block_id() {
            *self.get_block(block_id).signature.clone()
        } else {
            decl.signature()
        }
    }

    pub fn find_commands_by_predicate(
        &self,
        mut predicate: impl FnMut(&[u8]) -> bool,
        ignore_deprecated: bool,
    ) -> Vec<(DeclId, Vec<u8>, Option<String>, CommandType)> {
        let mut output = vec![];

        for scope_frame in self.delta.scope.iter().rev() {
            for overlay_id in scope_frame.active_overlays.iter().rev() {
                let overlay_frame = scope_frame.get_overlay(*overlay_id);

                for (name, decl_id) in &overlay_frame.decls {
                    if overlay_frame.visibility.is_decl_id_visible(decl_id) && predicate(name) {
                        let command = self.get_decl(*decl_id);
                        if ignore_deprecated && command.signature().category == Category::Removed {
                            continue;
                        }
                        output.push((
                            *decl_id,
                            name.clone(),
                            Some(command.description().to_string()),
                            command.command_type(),
                        ));
                    }
                }
            }
        }

        let mut permanent = self
            .permanent_state
            .find_commands_by_predicate(predicate, ignore_deprecated);

        output.append(&mut permanent);

        output
    }

    pub fn get_block(&self, block_id: BlockId) -> &Arc<Block> {
        let num_permanent_blocks = self.permanent_state.num_blocks();
        if block_id.get() < num_permanent_blocks {
            self.permanent_state.get_block(block_id)
        } else {
            self.delta
                .blocks
                .get(block_id.get() - num_permanent_blocks)
                .expect("internal error: missing block")
        }
    }

    pub fn get_module(&self, module_id: ModuleId) -> &Module {
        let num_permanent_modules = self.permanent_state.num_modules();
        if module_id.get() < num_permanent_modules {
            self.permanent_state.get_module(module_id)
        } else {
            self.delta
                .modules
                .get(module_id.get() - num_permanent_modules)
                .expect("internal error: missing module")
        }
    }

    pub fn get_block_mut(&mut self, block_id: BlockId) -> &mut Block {
        let num_permanent_blocks = self.permanent_state.num_blocks();
        if block_id.get() < num_permanent_blocks {
            panic!("Attempt to mutate a block that is in the permanent (immutable) state")
        } else {
            self.delta
                .blocks
                .get_mut(block_id.get() - num_permanent_blocks)
                .map(Arc::make_mut)
                .expect("internal error: missing block")
        }
    }

    /// Find the overlay corresponding to `name`.
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

    pub fn last_overlay_name(&self) -> &[u8] {
        let mut removed_overlays = vec![];

        for scope_frame in self.delta.scope.iter().rev() {
            if let Some(last_name) = scope_frame
                .active_overlay_names(&mut removed_overlays)
                .iter()
                .rev()
                .next_back()
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
                .rev()
                .next_back()
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
            self.add_overlay(
                name,
                origin,
                ResolvedImportPattern::new(vec![], vec![], vec![], vec![]),
                prefixed,
            );
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
                result.insert(decl_key.to_owned(), *decl_id);
            }
        }

        for scope_frame in self.delta.scope.iter() {
            if let Some(overlay_id) = scope_frame.find_overlay(name) {
                let overlay_frame = scope_frame.get_overlay(overlay_id);

                for (decl_key, decl_id) in &overlay_frame.decls {
                    result.insert(decl_key.to_owned(), *decl_id);
                }
            }
        }

        result
    }

    pub fn add_overlay(
        &mut self,
        name: Vec<u8>,
        origin: ModuleId,
        definitions: ResolvedImportPattern,
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
            OverlayId::new(last_scope_frame.overlays.len() - 1)
        };

        last_scope_frame
            .active_overlays
            .retain(|id| id != &overlay_id);
        last_scope_frame.active_overlays.push(overlay_id);

        self.move_predecls_to_overlay();

        self.use_decls(definitions.decls);
        self.use_modules(definitions.modules);

        let mut constants = vec![];

        for (name, const_vid) in definitions.constants {
            constants.push((name, const_vid));
        }

        for (name, const_val) in definitions.constant_values {
            let const_var_id =
                self.add_variable(name.clone(), Span::unknown(), const_val.get_type(), false);
            self.set_variable_const_val(const_var_id, const_val);
            constants.push((name, const_var_id));
        }
        self.use_variables(constants);
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

                self.use_decls(decls);
            }
        }
    }

    pub fn render(self) -> StateDelta {
        self.delta
    }

    pub fn build_desc(&self, spans: &[Span]) -> (String, String) {
        let comment_lines: Vec<&[u8]> = spans
            .iter()
            .map(|span| self.get_span_contents(*span))
            .collect();
        build_desc(&comment_lines)
    }

    pub fn find_block_by_span(&self, span: Span) -> Option<Arc<Block>> {
        for block in &self.delta.blocks {
            if Some(span) == block.span {
                return Some(block.clone());
            }
        }

        for block in self.permanent_state.blocks.iter() {
            if Some(span) == block.span {
                return Some(block.clone());
            }
        }

        None
    }

    pub fn find_module_by_span(&self, span: Span) -> Option<ModuleId> {
        for (id, module) in self.delta.modules.iter().enumerate() {
            if Some(span) == module.span {
                return Some(ModuleId::new(self.permanent_state.num_modules() + id));
            }
        }

        for (module_id, module) in self.permanent_state.modules.iter().enumerate() {
            if Some(span) == module.span {
                return Some(ModuleId::new(module_id));
            }
        }

        None
    }

    pub fn find_virtual_path(&self, name: &str) -> Option<&VirtualPath> {
        // Platform appropriate virtual path (slashes or backslashes)
        let virtual_path_name = Path::new(name);

        for (virtual_name, virtual_path) in self.delta.virtual_paths.iter().rev() {
            if Path::new(virtual_name) == virtual_path_name {
                return Some(virtual_path);
            }
        }

        for (virtual_name, virtual_path) in self.permanent_state.virtual_paths.iter().rev() {
            if Path::new(virtual_name) == virtual_path_name {
                return Some(virtual_path);
            }
        }

        None
    }

    pub fn get_virtual_path(&self, virtual_path_id: VirtualPathId) -> &(String, VirtualPath) {
        let num_permanent_virtual_paths = self.permanent_state.num_virtual_paths();
        if virtual_path_id.get() < num_permanent_virtual_paths {
            self.permanent_state.get_virtual_path(virtual_path_id)
        } else {
            self.delta
                .virtual_paths
                .get(virtual_path_id.get() - num_permanent_virtual_paths)
                .expect("internal error: missing virtual path")
        }
    }

    pub fn add_span(&mut self, span: Span) -> SpanId {
        let num_permanent_spans = self.permanent_state.spans.len();
        self.delta.spans.push(span);
        SpanId::new(num_permanent_spans + self.delta.spans.len() - 1)
    }
}

impl<'a> GetSpan for &'a StateWorkingSet<'a> {
    fn get_span(&self, span_id: SpanId) -> Span {
        let num_permanent_spans = self.permanent_state.num_spans();
        if span_id.get() < num_permanent_spans {
            self.permanent_state.get_span(span_id)
        } else {
            *self
                .delta
                .spans
                .get(span_id.get() - num_permanent_spans)
                .expect("internal error: missing span")
        }
    }
}

impl miette::SourceCode for &StateWorkingSet<'_> {
    fn read_span<'b>(
        &'b self,
        span: &miette::SourceSpan,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> Result<Box<dyn miette::SpanContents<'b> + 'b>, miette::MietteError> {
        let debugging = std::env::var("MIETTE_DEBUG").is_ok();
        if debugging {
            let finding_span = "Finding span in StateWorkingSet";
            dbg!(finding_span, span);
        }
        for cached_file in self.files() {
            let (filename, start, end) = (
                &cached_file.name,
                cached_file.covered_span.start,
                cached_file.covered_span.end,
            );
            if debugging {
                dbg!(&filename, start, end);
            }
            if span.offset() >= start && span.offset() + span.len() <= end {
                if debugging {
                    let found_file = "Found matching file";
                    dbg!(found_file);
                }
                let our_span = cached_file.covered_span;
                // We need to move to a local span because we're only reading
                // the specific file contents via self.get_span_contents.
                let local_span = (span.offset() - start, span.len()).into();
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
                if &**filename == "<cli>" {
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
                        (**filename).to_owned(),
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

/// Files being evaluated, arranged as a stack.
///
/// The current active file is on the top of the stack.
/// When a file source/import another file, the new file is pushed onto the stack.
/// Attempting to add files that are already in the stack (circular import) results in an error.
///
/// Note that file paths are compared without canonicalization, so the same
/// physical file may still appear multiple times under different paths.
/// This doesn't affect circular import detection though.
#[derive(Debug, Default)]
pub struct FileStack(Vec<PathBuf>);

impl FileStack {
    /// Creates an empty stack.
    pub fn new() -> Self {
        Self(vec![])
    }

    /// Creates a stack with a single file on top.
    ///
    /// This is a convenience method that creates an empty stack, then pushes the file onto it.
    /// It skips the circular import check and always succeeds.
    pub fn with_file(path: PathBuf) -> Self {
        Self(vec![path])
    }

    /// Adds a file to the stack.
    ///
    /// If the same file is already present in the stack, returns `ParseError::CircularImport`.
    pub fn push(&mut self, path: PathBuf, span: Span) -> Result<(), ParseError> {
        // Check for circular import.
        if let Some(i) = self.0.iter().rposition(|p| p == &path) {
            let filenames: Vec<String> = self.0[i..]
                .iter()
                .chain(std::iter::once(&path))
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            let msg = filenames.join("\nuses ");
            return Err(ParseError::CircularImport(msg, span));
        }

        self.0.push(path);
        Ok(())
    }

    /// Removes a file from the stack and returns its path, or None if the stack is empty.
    pub fn pop(&mut self) -> Option<PathBuf> {
        self.0.pop()
    }

    /// Returns the active file (that is, the file on the top of the stack), or None if the stack is empty.
    pub fn top(&self) -> Option<&Path> {
        self.0.last().map(PathBuf::as_path)
    }

    /// Returns the parent directory of the active file, or None if the stack is empty
    /// or the active file doesn't have a parent directory as part of its path.
    pub fn current_working_directory(&self) -> Option<&Path> {
        self.0.last().and_then(|path| path.parent())
    }
}
