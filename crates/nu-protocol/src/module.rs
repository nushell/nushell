use crate::{AliasId, BlockId, DeclId, Span};

use indexmap::IndexMap;

// TODO: Move the import pattern matching logic here from use/hide commands and
// parse_use/parse_hide

/// Collection of definitions that can be exported from a module
#[derive(Debug, Clone)]
pub struct Module {
    pub decls: IndexMap<Vec<u8>, DeclId>,
    pub aliases: IndexMap<Vec<u8>, AliasId>,
    pub env_block: Option<BlockId>,
    pub span: Option<Span>,
}

impl Module {
    pub fn new() -> Self {
        Module {
            decls: IndexMap::new(),
            aliases: IndexMap::new(),
            env_block: None,
            span: None,
        }
    }

    pub fn from_span(span: Span) -> Self {
        Module {
            decls: IndexMap::new(),
            aliases: IndexMap::new(),
            env_block: None,
            span: Some(span),
        }
    }

    pub fn add_decl(&mut self, name: Vec<u8>, decl_id: DeclId) -> Option<DeclId> {
        self.decls.insert(name, decl_id)
    }

    pub fn add_alias(&mut self, name: Vec<u8>, alias_id: AliasId) -> Option<AliasId> {
        self.aliases.insert(name, alias_id)
    }

    pub fn add_env_block(&mut self, block_id: BlockId) {
        self.env_block = Some(block_id);
    }

    pub fn extend(&mut self, other: &Module) {
        self.decls.extend(other.decls.clone());
        self.aliases.extend(other.aliases.clone());
    }

    pub fn is_empty(&self) -> bool {
        self.decls.is_empty() && self.aliases.is_empty()
    }

    pub fn get_decl_id(&self, name: &[u8]) -> Option<DeclId> {
        self.decls.get(name).copied()
    }

    pub fn get_alias_id(&self, name: &[u8]) -> Option<AliasId> {
        self.aliases.get(name).copied()
    }

    pub fn has_decl(&self, name: &[u8]) -> bool {
        self.decls.contains_key(name)
    }

    pub fn has_alias(&self, name: &[u8]) -> bool {
        self.aliases.contains_key(name)
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

    pub fn alias_name_with_head(&self, name: &[u8], head: &[u8]) -> Option<Vec<u8>> {
        if self.has_alias(name) {
            let mut new_name = head.to_vec();
            new_name.push(b' ');
            new_name.extend(name);
            Some(new_name)
        } else {
            None
        }
    }

    pub fn decls_with_head(&self, head: &[u8]) -> Vec<(Vec<u8>, DeclId)> {
        self.decls
            .iter()
            .map(|(name, id)| {
                let mut new_name = head.to_vec();
                new_name.push(b' ');
                new_name.extend(name);
                (new_name, *id)
            })
            .collect()
    }

    pub fn decl_names_with_head(&self, head: &[u8]) -> Vec<Vec<u8>> {
        self.decls
            .keys()
            .map(|name| {
                let mut new_name = head.to_vec();
                new_name.push(b' ');
                new_name.extend(name);
                new_name
            })
            .collect()
    }

    pub fn aliases_with_head(&self, head: &[u8]) -> Vec<(Vec<u8>, AliasId)> {
        self.aliases
            .iter()
            .map(|(name, id)| {
                let mut new_name = head.to_vec();
                new_name.push(b' ');
                new_name.extend(name);
                (new_name, *id)
            })
            .collect()
    }

    pub fn alias_names_with_head(&self, head: &[u8]) -> Vec<Vec<u8>> {
        self.aliases
            .keys()
            .map(|name| {
                let mut new_name = head.to_vec();
                new_name.push(b' ');
                new_name.extend(name);
                new_name
            })
            .collect()
    }

    pub fn decls(&self) -> Vec<(Vec<u8>, DeclId)> {
        self.decls
            .iter()
            .map(|(name, id)| (name.clone(), *id))
            .collect()
    }

    pub fn decl_names(&self) -> Vec<Vec<u8>> {
        self.decls.keys().cloned().collect()
    }

    pub fn alias_names(&self) -> Vec<Vec<u8>> {
        self.aliases.keys().cloned().collect()
    }

    pub fn aliases(&self) -> Vec<(Vec<u8>, AliasId)> {
        self.aliases
            .iter()
            .map(|(name, id)| (name.clone(), *id))
            .collect()
    }
}

impl Default for Module {
    fn default() -> Self {
        Self::new()
    }
}
