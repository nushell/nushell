use crate::{
    ast::ImportPatternMember, engine::StateWorkingSet, BlockId, DeclId, ModuleId, ParseError, Span,
};

use indexmap::IndexMap;

/// Collection of definitions that can be exported from a module
#[derive(Debug, Clone)]
pub struct Module {
    pub name: Vec<u8>,
    pub decls: IndexMap<Vec<u8>, DeclId>,
    pub submodules: IndexMap<Vec<u8>, ModuleId>,
    pub env_block: Option<BlockId>, // `export-env { ... }` block
    pub main: Option<DeclId>,       // `export def main`
    pub span: Option<Span>,
}

impl Module {
    pub fn new(name: Vec<u8>) -> Self {
        Module {
            name,
            decls: IndexMap::new(),
            submodules: IndexMap::new(),
            env_block: None,
            main: None,
            span: None,
        }
    }

    pub fn from_span(name: Vec<u8>, span: Span) -> Self {
        Module {
            name,
            decls: IndexMap::new(),
            submodules: IndexMap::new(),
            env_block: None,
            main: None,
            span: Some(span),
        }
    }

    pub fn name(&self) -> Vec<u8> {
        self.name.clone()
    }

    pub fn add_decl(&mut self, name: Vec<u8>, decl_id: DeclId) -> Option<DeclId> {
        self.decls.insert(name, decl_id)
    }

    pub fn add_submodule(&mut self, name: Vec<u8>, module_id: ModuleId) -> Option<ModuleId> {
        self.submodules.insert(name, module_id)
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

    pub fn resolve_import_pattern(
        &self,
        working_set: &StateWorkingSet,
        members: &[ImportPatternMember],
    ) -> (Vec<(Vec<u8>, DeclId)>, Vec<ParseError>) {
        let (head, rest) = if let Some((head, rest)) = members.split_first() {
            (head, rest)
        } else {
            let mut results = vec![];
            let mut errors = vec![];

            for (_, id) in &self.submodules {
                let submodule = working_set.get_module(*id);
                let (sub_results, sub_errors) = submodule.resolve_import_pattern(working_set, &[]);
                errors.extend(sub_errors);

                for (sub_name, sub_decl_id) in sub_results {
                    let mut new_name = self.name.clone();
                    new_name.push(b' ');
                    new_name.extend(sub_name);

                    results.push((new_name, sub_decl_id));
                }
            }

            results.extend(self.decls_with_head(&self.name));

            return (results, errors);
        };

        match head {
            ImportPatternMember::Name { name, span } => {
                if name == b"main" {
                    if let Some(main_decl_id) = self.main {
                        (vec![(self.name.clone(), main_decl_id)], vec![])
                    } else {
                        (vec![], vec![ParseError::ExportNotFound(*span)])
                    }
                } else if let Some(decl_id) = self.decls.get(name) {
                    (vec![(name.clone(), *decl_id)], vec![])
                } else if let Some(submodule_id) = self.submodules.get(name) {
                    let submodule = working_set.get_module(*submodule_id);
                    submodule.resolve_import_pattern(working_set, rest)
                } else {
                    (vec![], vec![ParseError::ExportNotFound(*span)])
                }
            }
            ImportPatternMember::Glob { .. } => {
                let mut results = vec![];
                let mut errors = vec![];

                for (_, id) in &self.submodules {
                    let submodule = working_set.get_module(*id);
                    let (sub_results, sub_errors) =
                        submodule.resolve_import_pattern(working_set, &[]);
                    errors.extend(sub_errors);
                    results.extend(sub_results);
                }

                results.extend(self.decls());

                (results, errors)
            }
            ImportPatternMember::List { names } => {
                let mut results = vec![];
                let mut errors = vec![];

                for (name, span) in names {
                    if name == b"main" {
                        if let Some(main_decl_id) = self.main {
                            results.push((self.name.clone(), main_decl_id));
                        } else {
                            errors.push(ParseError::ExportNotFound(*span));
                        }
                    } else if let Some(decl_id) = self.decls.get(name) {
                        results.push((name.clone(), *decl_id));
                    } else {
                        errors.push(ParseError::ExportNotFound(*span));
                    }
                }

                (results, errors)
            }
        }
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
