use crate::{BlockId, DeclId, Span};

use indexmap::IndexMap;

/// Collection of definitions that can be exported from a module
#[derive(Debug, Clone)]
pub struct Module {
    pub name: Vec<u8>,
    pub decls: IndexMap<Vec<u8>, DeclId>,
    pub env_block: Option<BlockId>, // `export-env { ... }` block
    pub main: Option<DeclId>,       // `export def main`
    pub span: Option<Span>,
}

impl Module {
    pub fn new(name: Vec<u8>) -> Self {
        Module {
            name,
            decls: IndexMap::new(),
            env_block: None,
            main: None,
            span: None,
        }
    }

    pub fn from_span(name: Vec<u8>, span: Span) -> Self {
        Module {
            name,
            decls: IndexMap::new(),
            env_block: None,
            main: None,
            span: Some(span),
        }
    }

    pub fn add_decl(&mut self, name: Vec<u8>, decl_id: DeclId) -> Option<DeclId> {
        self.decls.insert(name, decl_id)
    }

    pub fn add_env_block(&mut self, block_id: BlockId) {
        self.env_block = Some(block_id);
    }

    pub fn extend(&mut self, other: &Module) {
        self.decls.extend(other.decls.clone());
    }

    pub fn is_empty(&self) -> bool {
        self.decls.is_empty()
    }

    pub fn get_decl_id(&self, name: &[u8]) -> Option<DeclId> {
        self.decls.get(name).copied()
    }

    pub fn has_decl(&self, name: &[u8]) -> bool {
        if name == self.name && self.main.is_some() {
            return true;
        }

        self.decls.contains_key(name)
    }

    pub fn decl_name_with_head(&self, name: &[u8], head: &[u8]) -> Option<Vec<u8>> {
        if self.has_decl(name) {
            let mut new_name = head.to_vec();
            new_name.push(b' ');
            new_name.extend(name);
            Some(new_name)
        } else {
            None
        }
    }

    pub fn decls_with_head(&self, head: &[u8]) -> Vec<(Vec<u8>, DeclId)> {
        let mut result: Vec<(Vec<u8>, DeclId)> = self
            .decls
            .iter()
            .map(|(name, id)| {
                let mut new_name = head.to_vec();
                new_name.push(b' ');
                new_name.extend(name);
                (new_name, *id)
            })
            .collect();

        if let Some(decl_id) = self.main {
            result.push((self.name.clone(), decl_id));
        }

        result
    }

    pub fn decl_names_with_head(&self, head: &[u8]) -> Vec<Vec<u8>> {
        let mut result: Vec<Vec<u8>> = self
            .decls
            .keys()
            .map(|name| {
                let mut new_name = head.to_vec();
                new_name.push(b' ');
                new_name.extend(name);
                new_name
            })
            .collect();

        if self.main.is_some() {
            result.push(self.name.clone());
        }

        result
    }

    pub fn decls(&self) -> Vec<(Vec<u8>, DeclId)> {
        let mut result: Vec<(Vec<u8>, DeclId)> = self
            .decls
            .iter()
            .map(|(name, id)| (name.clone(), *id))
            .collect();

        if let Some(decl_id) = self.main {
            result.push((self.name.clone(), decl_id));
        }

        result
    }

    pub fn decl_names(&self) -> Vec<Vec<u8>> {
        let mut result: Vec<Vec<u8>> = self.decls.keys().cloned().collect();

        if self.main.is_some() {
            result.push(self.name.clone());
        }

        result
    }
}
