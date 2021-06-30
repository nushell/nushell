use crate::Span;
use std::{collections::HashMap, sync::Arc};

pub struct ParserState {
    files: Vec<(String, Vec<u8>)>,
}

pub enum VarLocation {
    CurrentScope,
    OuterScope,
}

#[derive(Clone, Copy)]
pub enum Type {}

struct ScopeFrame {
    vars: HashMap<String, Type>,
}

impl ScopeFrame {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }
}

pub struct ParserWorkingSet {
    files: Vec<(String, Vec<u8>)>,
    permanent_state: Option<Arc<ParserState>>,
    scope: Vec<ScopeFrame>,
}

impl Default for ParserState {
    fn default() -> Self {
        Self::new()
    }
}

impl ParserState {
    pub fn new() -> Self {
        Self { files: vec![] }
    }

    pub fn merge_working_set(this: &mut Arc<ParserState>, mut working_set: ParserWorkingSet) {
        // Remove the working set's reference to the permanent state so we can safely take a mutable reference
        working_set.permanent_state = None;

        // Take the mutable reference and extend the permanent state from the working set
        if let Some(this) = std::sync::Arc::<ParserState>::get_mut(this) {
            this.files.extend(working_set.files);
        } else {
            panic!("Internal error: merging working set should always succeed");
        }
    }

    pub fn num_files(&self) -> usize {
        self.files.len()
    }

    pub(crate) fn add_file(&mut self, filename: String, contents: Vec<u8>) -> usize {
        self.files.push((filename, contents));

        self.num_files() - 1
    }

    pub(crate) fn get_file_contents(&self, idx: usize) -> &[u8] {
        &self.files[idx].1
    }
}

impl ParserWorkingSet {
    pub fn new(permanent_state: Option<Arc<ParserState>>) -> Self {
        Self {
            files: vec![],
            permanent_state,
            scope: vec![],
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

    pub fn enter_scope(&mut self) {
        self.scope.push(ScopeFrame::new());
    }

    pub fn exit_scope(&mut self) {
        self.scope.push(ScopeFrame::new());
    }

    pub fn find_variable(&self, name: &str) -> Option<(VarLocation, Type)> {
        for scope in self.scope.iter().rev().enumerate() {
            if let Some(result) = scope.1.vars.get(name) {
                if scope.0 == 0 {
                    // Top level
                    return Some((VarLocation::CurrentScope, result.clone()));
                } else {
                    return Some((VarLocation::OuterScope, result.clone()));
                }
            }
        }

        None
    }
}

fn main() {}

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
        let parent_id = parser_state.add_file("test.nu".into(), vec![]);
        let mut parser_state = Arc::new(parser_state);

        let mut working_set = ParserWorkingSet::new(Some(parser_state.clone()));
        let working_set_id = working_set.add_file("child.nu".into(), vec![]);

        ParserState::merge_working_set(&mut parser_state, working_set);

        assert_eq!(parser_state.num_files(), 2);
        assert_eq!(&parser_state.files[0].0, "test.nu");
        assert_eq!(&parser_state.files[1].0, "child.nu");
    }
}
