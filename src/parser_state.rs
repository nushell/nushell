use crate::{Signature, Span};
use std::{collections::HashMap, sync::Arc};

pub struct ParserState {
    files: Vec<(String, Vec<u8>)>,
    vars: Vec<Type>,
    decls: Vec<Signature>,
}

#[derive(Clone, Copy, Debug)]
pub enum Type {
    Int,
    Unknown,
}

pub type VarId = usize;
pub type DeclId = usize;

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
            vars: vec![],
            decls: vec![],
        }
    }

    pub fn merge_working_set(this: &mut Arc<ParserState>, mut working_set: ParserWorkingSet) {
        // Remove the working set's reference to the permanent state so we can safely take a mutable reference
        working_set.permanent_state = None;

        // Take the mutable reference and extend the permanent state from the working set
        if let Some(this) = std::sync::Arc::<ParserState>::get_mut(this) {
            this.files.extend(working_set.files);
            this.decls.extend(working_set.decls);
            this.vars.extend(working_set.vars);

            //FIXME: add scope frame merging
        } else {
            panic!("Internal error: merging working set should always succeed");
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

    pub fn get_var(&self, var_id: VarId) -> Option<&Type> {
        self.vars.get(var_id)
    }

    pub fn get_decl(&self, decl_id: DeclId) -> Option<&Signature> {
        self.decls.get(decl_id)
    }

    #[allow(unused)]
    pub(crate) fn add_file(&mut self, filename: String, contents: Vec<u8>) -> usize {
        self.files.push((filename, contents));

        self.num_files() - 1
    }

    pub(crate) fn get_file_contents(&self, idx: usize) -> &[u8] {
        &self.files[idx].1
    }
}

pub struct ParserWorkingSet {
    files: Vec<(String, Vec<u8>)>,
    vars: Vec<Type>,       // indexed by VarId
    decls: Vec<Signature>, // indexed by DeclId
    permanent_state: Option<Arc<ParserState>>,
    scope: Vec<ScopeFrame>,
}

impl ParserWorkingSet {
    pub fn new(permanent_state: Option<Arc<ParserState>>) -> Self {
        Self {
            files: vec![],
            vars: vec![],
            decls: vec![],
            permanent_state,
            scope: vec![ScopeFrame::new()],
        }
    }

    pub fn num_files(&self) -> usize {
        let parent_len = if let Some(permanent_state) = &self.permanent_state {
            permanent_state.num_files()
        } else {
            0
        };

        self.files.len() + parent_len
    }

    pub fn add_decl(&mut self, name: Vec<u8>, sig: Signature) -> DeclId {
        let scope_frame = self
            .scope
            .last_mut()
            .expect("internal error: missing required scope frame");

        self.decls.push(sig);
        let decl_id = self.decls.len() - 1;

        scope_frame.decls.insert(name, decl_id);

        decl_id
    }

    pub fn add_file(&mut self, filename: String, contents: Vec<u8>) -> usize {
        self.files.push((filename, contents));

        self.num_files() - 1
    }

    pub fn get_span_contents(&self, span: Span) -> &[u8] {
        if let Some(permanent_state) = &self.permanent_state {
            let num_permanent_files = permanent_state.num_files();
            if span.file_id < num_permanent_files {
                &permanent_state.get_file_contents(span.file_id)[span.start..span.end]
            } else {
                &self.files[span.file_id - num_permanent_files].1[span.start..span.end]
            }
        } else {
            &self.files[span.file_id].1[span.start..span.end]
        }
    }

    pub fn get_file_contents(&self, file_id: usize) -> &[u8] {
        if let Some(permanent_state) = &self.permanent_state {
            let num_permanent_files = permanent_state.num_files();
            if file_id < num_permanent_files {
                &permanent_state.get_file_contents(file_id)
            } else {
                &self.files[file_id - num_permanent_files].1
            }
        } else {
            &self.files[file_id].1
        }
    }

    pub fn enter_scope(&mut self) {
        self.scope.push(ScopeFrame::new());
    }

    pub fn exit_scope(&mut self) {
        self.scope.push(ScopeFrame::new());
    }

    pub fn find_decl(&self, name: &[u8]) -> Option<DeclId> {
        for scope in self.scope.iter().rev().enumerate() {
            if let Some(decl_id) = scope.1.decls.get(name) {
                return Some(*decl_id);
            }
        }

        None
    }

    pub fn next_var_id(&self) -> VarId {
        if let Some(permanent_state) = &self.permanent_state {
            let num_permanent_vars = permanent_state.num_vars();
            num_permanent_vars + self.vars.len()
        } else {
            self.vars.len()
        }
    }

    pub fn find_variable(&self, name: &[u8]) -> Option<VarId> {
        for scope in self.scope.iter().rev().enumerate() {
            if let Some(var_id) = scope.1.vars.get(name) {
                return Some(*var_id);
            }
        }

        None
    }

    pub fn add_variable(&mut self, name: Vec<u8>, ty: Type) -> VarId {
        let next_id = self.next_var_id();

        let last = self
            .scope
            .last_mut()
            .expect("internal error: missing stack frame");

        last.vars.insert(name, next_id);

        self.vars.insert(next_id, ty);

        next_id
    }

    pub fn get_variable(&self, var_id: VarId) -> Option<&Type> {
        if let Some(permanent_state) = &self.permanent_state {
            let num_permanent_vars = permanent_state.num_vars();
            if var_id < num_permanent_vars {
                permanent_state.get_var(var_id)
            } else {
                self.vars.get(var_id - num_permanent_vars)
            }
        } else {
            self.vars.get(var_id)
        }
    }

    pub fn get_decl(&self, decl_id: DeclId) -> Option<&Signature> {
        if let Some(permanent_state) = &self.permanent_state {
            let num_permanent_decls = permanent_state.num_decls();
            if decl_id < num_permanent_decls {
                permanent_state.get_decl(decl_id)
            } else {
                self.decls.get(decl_id - num_permanent_decls)
            }
        } else {
            self.decls.get(decl_id)
        }
    }
}

#[cfg(test)]
mod parser_state_tests {
    use super::*;

    #[test]
    fn add_file_gives_id() {
        let mut parser_state = ParserWorkingSet::new(Some(Arc::new(ParserState::new())));
        let id = parser_state.add_file("test.nu".into(), vec![]);

        assert_eq!(id, 0);
    }

    #[test]
    fn add_file_gives_id_including_parent() {
        let mut parser_state = ParserState::new();
        let parent_id = parser_state.add_file("test.nu".into(), vec![]);

        let mut working_set = ParserWorkingSet::new(Some(Arc::new(parser_state)));
        let working_set_id = working_set.add_file("child.nu".into(), vec![]);

        assert_eq!(parent_id, 0);
        assert_eq!(working_set_id, 1);
    }

    #[test]
    fn merge_states() {
        let mut parser_state = ParserState::new();
        parser_state.add_file("test.nu".into(), vec![]);
        let mut parser_state = Arc::new(parser_state);

        let mut working_set = ParserWorkingSet::new(Some(parser_state.clone()));
        working_set.add_file("child.nu".into(), vec![]);

        ParserState::merge_working_set(&mut parser_state, working_set);

        assert_eq!(parser_state.num_files(), 2);
        assert_eq!(&parser_state.files[0].0, "test.nu");
        assert_eq!(&parser_state.files[1].0, "child.nu");
    }
}
