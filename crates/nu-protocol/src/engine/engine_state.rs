use super::Command;
use crate::{ast::Block, BlockId, DeclId, Span, Type, VarId};
use core::panic;
use std::{collections::HashMap, ops::Range, slice::Iter};

pub struct EngineState {
    files: Vec<(String, usize, usize)>,
    file_contents: Vec<u8>,
    vars: Vec<Type>,
    decls: Vec<Box<dyn Command>>,
    blocks: Vec<Block>,
    scope: Vec<ScopeFrame>,
}

#[derive(Debug)]
struct ScopeFrame {
    vars: HashMap<Vec<u8>, VarId>,
    decls: HashMap<Vec<u8>, DeclId>,
    aliases: HashMap<Vec<u8>, Vec<Span>>,
}

impl ScopeFrame {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            decls: HashMap::new(),
            aliases: HashMap::new(),
        }
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

    pub fn find_decl(&self, name: &[u8]) -> Option<DeclId> {
        for scope in self.scope.iter().rev() {
            if let Some(decl_id) = scope.decls.get(name) {
                return Some(*decl_id);
            }
        }

        None
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
    vars: Vec<Type>,              // indexed by VarId
    decls: Vec<Box<dyn Command>>, // indexed by DeclId
    blocks: Vec<Block>,           // indexed by BlockId
    scope: Vec<ScopeFrame>,
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

    pub fn add_block(&mut self, block: Block) -> BlockId {
        self.delta.blocks.push(block);

        self.num_blocks() - 1
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
        for scope in self.delta.scope.iter().rev() {
            if let Some(decl_id) = scope.decls.get(name) {
                return Some(*decl_id);
            }
        }

        for scope in self.permanent_state.scope.iter().rev() {
            if let Some(decl_id) = scope.decls.get(name) {
                return Some(*decl_id);
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

impl<'a> codespan_reporting::files::Files<'a> for StateWorkingSet<'a> {
    type FileId = usize;

    type Name = String;

    type Source = String;

    fn name(&'a self, id: Self::FileId) -> Result<Self::Name, codespan_reporting::files::Error> {
        Ok(self.get_filename(id))
    }

    fn source(
        &'a self,
        id: Self::FileId,
    ) -> Result<Self::Source, codespan_reporting::files::Error> {
        Ok(self.get_file_source(id))
    }

    fn line_index(
        &'a self,
        id: Self::FileId,
        byte_index: usize,
    ) -> Result<usize, codespan_reporting::files::Error> {
        let source = self.get_file_source(id);

        let mut count = 0;

        for byte in source.bytes().enumerate() {
            if byte.0 == byte_index {
                // println!("count: {} for file: {} index: {}", count, id, byte_index);
                return Ok(count);
            }
            if byte.1 == b'\n' {
                count += 1;
            }
        }

        // println!("count: {} for file: {} index: {}", count, id, byte_index);
        Ok(count)
    }

    fn line_range(
        &'a self,
        id: Self::FileId,
        line_index: usize,
    ) -> Result<Range<usize>, codespan_reporting::files::Error> {
        let source = self.get_file_source(id);

        let mut count = 0;

        let mut start = Some(0);
        let mut end = None;

        for byte in source.bytes().enumerate() {
            #[allow(clippy::comparison_chain)]
            if count > line_index {
                let start = start.expect("internal error: couldn't find line");
                let end = end.expect("internal error: couldn't find line");

                // println!(
                //     "Span: {}..{} for fileid: {} index: {}",
                //     start, end, id, line_index
                // );
                return Ok(start..end);
            } else if count == line_index {
                end = Some(byte.0 + 1);
            }

            #[allow(clippy::comparison_chain)]
            if byte.1 == b'\n' {
                count += 1;
                if count > line_index {
                    break;
                } else if count == line_index {
                    start = Some(byte.0 + 1);
                }
            }
        }

        match (start, end) {
            (Some(start), Some(end)) => {
                // println!(
                //     "Span: {}..{} for fileid: {} index: {}",
                //     start, end, id, line_index
                // );
                Ok(start..end)
            }
            _ => Err(codespan_reporting::files::Error::FileMissing),
        }
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
