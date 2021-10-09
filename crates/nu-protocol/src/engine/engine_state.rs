use super::Command;
use crate::{ast::Block, BlockId, DeclId, Example, Signature, Span, Type, VarId};
use core::panic;
use std::{
    collections::{HashMap, HashSet},
    slice::Iter,
};

pub struct EngineState {
    files: Vec<(String, usize, usize)>,
    file_contents: Vec<u8>,
    vars: Vec<Type>,
    decls: Vec<Box<dyn Command>>,
    blocks: Vec<Block>,
    pub scope: Vec<ScopeFrame>,
}

#[derive(Debug)]
pub struct ScopeFrame {
    pub vars: HashMap<Vec<u8>, VarId>,
    decls: HashMap<Vec<u8>, DeclId>,
    aliases: HashMap<Vec<u8>, Vec<Span>>,
    modules: HashMap<Vec<u8>, BlockId>,
    hiding: HashSet<DeclId>,
}

impl ScopeFrame {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            decls: HashMap::new(),
            aliases: HashMap::new(),
            modules: HashMap::new(),
            hiding: HashSet::new(),
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

impl Default for EngineState {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineState {
    pub fn new() -> Self {
        Self {
            files: vec![],
            file_contents: vec![],
            vars: vec![],
            decls: vec![],
            blocks: vec![],
            scope: vec![ScopeFrame::new()],
        }
    }

    pub fn merge_delta(this: &mut EngineState, mut delta: StateDelta) {
        // Take the mutable reference and extend the permanent state from the working set
        this.files.extend(delta.files);
        this.file_contents.extend(delta.file_contents);
        this.decls.extend(delta.decls);
        this.vars.extend(delta.vars);
        this.blocks.extend(delta.blocks);

        if let Some(last) = this.scope.last_mut() {
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
            for item in first.modules.into_iter() {
                last.modules.insert(item.0, item.1);
            }
            for item in first.hiding.into_iter() {
                last.hiding.insert(item);
            }
        }
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
        let string = String::from_utf8_lossy(&self.file_contents);
        println!("{}", string);
    }

    pub fn find_decl(&self, name: &[u8]) -> Option<DeclId> {
        let mut hiding: HashSet<DeclId> = HashSet::new();

        for scope in self.scope.iter().rev() {
            hiding.extend(&scope.hiding);

            if let Some(decl_id) = scope.decls.get(name) {
                if !hiding.contains(decl_id) {
                    return Some(*decl_id);
                }
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
        &self.file_contents[span.start..span.end]
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

    pub fn get_signatures(&self) -> Vec<Signature> {
        let mut output = vec![];
        for decl in self.decls.iter() {
            if decl.get_block_id().is_none() {
                let mut signature = (*decl).signature();
                signature.usage = decl.usage().to_string();
                signature.extra_usage = decl.extra_usage().to_string();

                output.push(signature);
            }
        }

        output
    }

    pub fn get_signatures_with_examples(&self) -> Vec<(Signature, Vec<Example>)> {
        let mut output = vec![];
        for decl in self.decls.iter() {
            if decl.get_block_id().is_none() {
                let mut signature = (*decl).signature();
                signature.usage = decl.usage().to_string();
                signature.extra_usage = decl.extra_usage().to_string();

                output.push((signature, decl.examples()));
            }
        }

        output
    }

    pub fn get_block(&self, block_id: BlockId) -> &Block {
        self.blocks
            .get(block_id)
            .expect("internal error: missing block")
    }

    pub fn next_span_start(&self) -> usize {
        self.file_contents.len()
    }

    pub fn files(&self) -> Iter<(String, usize, usize)> {
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
                let output =
                    String::from_utf8_lossy(&self.file_contents[file.1 .1..file.1 .2]).to_string();

                return output;
            }
        }

        "<unknown>".into()
    }

    #[allow(unused)]
    pub(crate) fn add_file(&mut self, filename: String, contents: Vec<u8>) -> usize {
        let next_span_start = self.next_span_start();

        self.file_contents.extend(&contents);

        let next_span_end = self.next_span_start();

        self.files.push((filename, next_span_start, next_span_end));

        self.num_files() - 1
    }
}

pub struct StateWorkingSet<'a> {
    pub permanent_state: &'a EngineState,
    pub delta: StateDelta,
}

pub struct StateDelta {
    files: Vec<(String, usize, usize)>,
    pub(crate) file_contents: Vec<u8>,
    vars: Vec<Type>,                    // indexed by VarId
    decls: Vec<Box<dyn Command>>,       // indexed by DeclId
    blocks: Vec<Block>,                 // indexed by BlockId
    predecls: HashMap<Vec<u8>, DeclId>, // this should get erased after every def call
    pub scope: Vec<ScopeFrame>,
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
                predecls: HashMap::new(),
                blocks: vec![],
                scope: vec![ScopeFrame::new()],
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

        decl_id
    }

    pub fn add_predecl(&mut self, decl: Box<dyn Command>) -> Option<DeclId> {
        let name = decl.name().as_bytes().to_vec();

        self.delta.decls.push(decl);
        let decl_id = self.num_decls() - 1;

        self.delta.predecls.insert(name, decl_id)
    }

    pub fn merge_predecl(&mut self, name: &[u8]) -> Option<DeclId> {
        if let Some(decl_id) = self.delta.predecls.remove(name) {
            let scope_frame = self
                .delta
                .scope
                .last_mut()
                .expect("internal error: missing required scope frame");

            scope_frame.decls.insert(name.into(), decl_id);

            return Some(decl_id);
        }

        None
    }

    pub fn hide_decl(&mut self, name: &[u8]) -> Option<DeclId> {
        let mut hiding: HashSet<DeclId> = HashSet::new();

        // Since we can mutate scope frames in delta, remove the id directly
        for scope in self.delta.scope.iter_mut().rev() {
            hiding.extend(&scope.hiding);

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
            hiding.extend(&scope.hiding);

            if let Some(decl_id) = scope.decls.get(name) {
                if !hiding.contains(decl_id) {
                    // Hide decl only if it's not already hidden
                    last_scope_frame.hiding.insert(*decl_id);
                    return Some(*decl_id);
                }
            }
        }

        None
    }

    pub fn add_block(&mut self, block: Block) -> BlockId {
        self.delta.blocks.push(block);

        self.num_blocks() - 1
    }

    pub fn add_module(&mut self, name: &str, block: Block) -> BlockId {
        let name = name.as_bytes().to_vec();

        self.delta.blocks.push(block);
        let block_id = self.num_blocks() - 1;

        let scope_frame = self
            .delta
            .scope
            .last_mut()
            .expect("internal error: missing required scope frame");

        scope_frame.modules.insert(name, block_id);

        block_id
    }

    pub fn activate_overlay(&mut self, overlay: Vec<(Vec<u8>, DeclId)>) {
        let scope_frame = self
            .delta
            .scope
            .last_mut()
            .expect("internal error: missing required scope frame");

        for (name, decl_id) in overlay {
            scope_frame.decls.insert(name, decl_id);
        }
    }

    pub fn next_span_start(&self) -> usize {
        self.permanent_state.next_span_start() + self.delta.file_contents.len()
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

        self.delta.file_contents.extend(contents);

        let next_span_end = self.next_span_start();

        self.delta
            .files
            .push((filename, next_span_start, next_span_end));

        self.num_files() - 1
    }

    pub fn get_span_contents(&self, span: Span) -> &[u8] {
        let permanent_end = self.permanent_state.next_span_start();
        if permanent_end <= span.start {
            &self.delta.file_contents[(span.start - permanent_end)..(span.end - permanent_end)]
        } else {
            &self.permanent_state.file_contents[span.start..span.end]
        }
    }

    pub fn enter_scope(&mut self) {
        self.delta.enter_scope();
    }

    pub fn exit_scope(&mut self) {
        self.delta.exit_scope();
    }

    pub fn find_decl(&self, name: &[u8]) -> Option<DeclId> {
        let mut hiding: HashSet<DeclId> = HashSet::new();

        if let Some(decl_id) = self.delta.predecls.get(name) {
            return Some(*decl_id);
        }

        for scope in self.delta.scope.iter().rev() {
            hiding.extend(&scope.hiding);

            if let Some(decl_id) = scope.decls.get(name) {
                return Some(*decl_id);
            }
        }

        for scope in self.permanent_state.scope.iter().rev() {
            hiding.extend(&scope.hiding);

            if let Some(decl_id) = scope.decls.get(name) {
                if !hiding.contains(decl_id) {
                    return Some(*decl_id);
                }
            }
        }

        None
    }

    pub fn find_module(&self, name: &[u8]) -> Option<BlockId> {
        for scope in self.delta.scope.iter().rev() {
            if let Some(block_id) = scope.modules.get(name) {
                return Some(*block_id);
            }
        }

        for scope in self.permanent_state.scope.iter().rev() {
            if let Some(block_id) = scope.modules.get(name) {
                return Some(*block_id);
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
    fn merge_states() {
        let mut engine_state = EngineState::new();
        engine_state.add_file("test.nu".into(), vec![]);

        let delta = {
            let mut working_set = StateWorkingSet::new(&engine_state);
            working_set.add_file("child.nu".into(), &[]);
            working_set.render()
        };

        EngineState::merge_delta(&mut engine_state, delta);

        assert_eq!(engine_state.num_files(), 2);
        assert_eq!(&engine_state.files[0].0, "test.nu");
        assert_eq!(&engine_state.files[1].0, "child.nu");
    }
}
