use crate::{parser::Block, Declaration, Span};
use core::panic;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ParserState {
    files: Vec<(String, usize, usize)>,
    file_contents: Vec<u8>,
    vars: Vec<Type>,
    decls: Vec<Declaration>,
    blocks: Vec<Block>,
    scope: Vec<ScopeFrame>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Int,
    Bool,
    String,
    Block,
    ColumnPath,
    Duration,
    FilePath,
    Filesize,
    List(Box<Type>),
    Number,
    Table,
    Unknown,
}

pub type VarId = usize;
pub type DeclId = usize;
pub type BlockId = usize;

#[derive(Debug)]
struct ScopeFrame {
    vars: HashMap<Vec<u8>, VarId>,
    decls: HashMap<Vec<u8>, DeclId>,
}

impl ScopeFrame {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            decls: HashMap::new(),
        }
    }
}

impl Default for ParserState {
    fn default() -> Self {
        Self::new()
    }
}

impl ParserState {
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

    pub fn merge_delta(this: &mut ParserState, mut delta: ParserDelta) {
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
            println!("decl{}: {:?}", decl.0, decl.1);
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

    pub fn get_decl(&self, decl_id: DeclId) -> &Declaration {
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

    #[allow(unused)]
    pub(crate) fn add_file(&mut self, filename: String, contents: Vec<u8>) -> usize {
        let next_span_start = self.next_span_start();

        self.file_contents.extend(&contents);

        let next_span_end = self.next_span_start();

        self.files.push((filename, next_span_start, next_span_end));

        self.num_files() - 1
    }
}

#[derive(Debug)]
pub struct ParserWorkingSet<'a> {
    permanent_state: &'a ParserState,
    pub delta: ParserDelta,
}

#[derive(Debug)]
pub struct ParserDelta {
    files: Vec<(String, usize, usize)>,
    pub(crate) file_contents: Vec<u8>,
    vars: Vec<Type>,         // indexed by VarId
    decls: Vec<Declaration>, // indexed by DeclId
    blocks: Vec<Block>,      // indexed by BlockId
    scope: Vec<ScopeFrame>,
}

impl ParserDelta {
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

impl<'a> ParserWorkingSet<'a> {
    pub fn new(permanent_state: &'a ParserState) -> Self {
        Self {
            delta: ParserDelta {
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

    pub fn add_decl(&mut self, decl: Declaration) -> DeclId {
        let name = decl.signature.name.as_bytes().to_vec();

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

    pub fn update_decl(&mut self, decl_id: usize, block: Option<BlockId>) {
        let decl = self.get_decl_mut(decl_id);
        decl.body = block;
    }

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

    pub fn get_decl(&self, decl_id: DeclId) -> &Declaration {
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

    pub fn get_decl_mut(&mut self, decl_id: DeclId) -> &mut Declaration {
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

    pub fn render(self) -> ParserDelta {
        self.delta
    }
}

#[cfg(test)]
mod parser_state_tests {
    use super::*;

    #[test]
    fn add_file_gives_id() {
        let parser_state = ParserState::new();
        let mut parser_state = ParserWorkingSet::new(&parser_state);
        let id = parser_state.add_file("test.nu".into(), &[]);

        assert_eq!(id, 0);
    }

    #[test]
    fn add_file_gives_id_including_parent() {
        let mut parser_state = ParserState::new();
        let parent_id = parser_state.add_file("test.nu".into(), vec![]);

        let mut working_set = ParserWorkingSet::new(&parser_state);
        let working_set_id = working_set.add_file("child.nu".into(), &[]);

        assert_eq!(parent_id, 0);
        assert_eq!(working_set_id, 1);
    }

    #[test]
    fn merge_states() {
        let mut parser_state = ParserState::new();
        parser_state.add_file("test.nu".into(), vec![]);

        let delta = {
            let mut working_set = ParserWorkingSet::new(&parser_state);
            working_set.add_file("child.nu".into(), &[]);
            working_set.render()
        };

        ParserState::merge_delta(&mut parser_state, delta);

        assert_eq!(parser_state.num_files(), 2);
        assert_eq!(&parser_state.files[0].0, "test.nu");
        assert_eq!(&parser_state.files[1].0, "child.nu");
    }
}
