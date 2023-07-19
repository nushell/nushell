use crate::{
    ast::ImportPatternMember, engine::StateWorkingSet, BlockId, DeclId, ModuleId, ParseError, Span,
    VarId,
};

use indexmap::IndexMap;

pub struct ResolvedImportPattern {
    pub decls: Vec<(Vec<u8>, DeclId)>,
    pub modules: Vec<(Vec<u8>, ModuleId)>,
    pub variables: Vec<(Vec<u8>, VarId)>,
}

impl ResolvedImportPattern {
    pub fn new(
        decls: Vec<(Vec<u8>, DeclId)>,
        modules: Vec<(Vec<u8>, ModuleId)>,
        variables: Vec<(Vec<u8>, VarId)>,
    ) -> Self {
        ResolvedImportPattern {
            decls,
            modules,
            variables,
        }
    }
}

/// Collection of definitions that can be exported from a module
#[derive(Debug, Clone)]
pub struct Module {
    pub name: Vec<u8>,
    pub decls: IndexMap<Vec<u8>, DeclId>,
    pub submodules: IndexMap<Vec<u8>, ModuleId>,
    pub variables: IndexMap<Vec<u8>, VarId>,
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
            variables: IndexMap::new(),
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
            variables: IndexMap::new(),
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

    pub fn add_variable(&mut self, name: Vec<u8>, var_id: VarId) -> Option<VarId> {
        self.variables.insert(name, var_id)
    }

    pub fn add_env_block(&mut self, block_id: BlockId) {
        self.env_block = Some(block_id);
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
        self_id: ModuleId,
        members: &[ImportPatternMember],
        name_override: Option<&[u8]>, // name under the module was stored (doesn't have to be the
                                      // same as self.name)
    ) -> (ResolvedImportPattern, Vec<ParseError>) {
        let final_name = name_override.unwrap_or(&self.name).to_vec();

        let (head, rest) = if let Some((head, rest)) = members.split_first() {
            (head, rest)
        } else {
            // Import pattern was just name without any members
            let mut decls = vec![];
            let mut vars = vec![];
            let mut errors = vec![];

            for (_, id) in &self.submodules {
                let submodule = working_set.get_module(*id);
                let (sub_results, sub_errors) =
                    submodule.resolve_import_pattern(working_set, *id, &[], None);
                errors.extend(sub_errors);

                for (sub_name, sub_decl_id) in sub_results.decls {
                    let mut new_name = final_name.clone();
                    new_name.push(b' ');
                    new_name.extend(sub_name);

                    decls.push((new_name, sub_decl_id));
                }

                for (sub_name, sub_var_id) in sub_results.variables {
                    let mut new_name = final_name.clone();
                    new_name.push(b' ');
                    new_name.extend(sub_name);

                    vars.push((new_name, sub_var_id));
                }
            }

            decls.extend(self.decls_with_head(&final_name));
            vars.extend(self.vars_with_head(&final_name));

            return (
                ResolvedImportPattern::new(decls, vec![(final_name, self_id)], vars),
                errors,
            );
        };

        match head {
            ImportPatternMember::Name { name, span } => {
                if name == b"main" {
                    if let Some(main_decl_id) = self.main {
                        (
                            ResolvedImportPattern::new(
                                vec![(final_name, main_decl_id)],
                                vec![],
                                vec![],
                            ),
                            vec![],
                        )
                    } else {
                        (
                            ResolvedImportPattern::new(vec![], vec![], vec![]),
                            vec![ParseError::ExportNotFound(*span)],
                        )
                    }
                } else if let Some(decl_id) = self.decls.get(name) {
                    (
                        ResolvedImportPattern::new(vec![(name.clone(), *decl_id)], vec![], vec![]),
                        vec![],
                    )
                } else if let Some(var_id) = self.variables.get(name) {
                    (
                        ResolvedImportPattern::new(vec![], vec![], vec![(name.clone(), *var_id)]),
                        vec![],
                    )
                } else if let Some(submodule_id) = self.submodules.get(name) {
                    let submodule = working_set.get_module(*submodule_id);
                    submodule.resolve_import_pattern(working_set, *submodule_id, rest, None)
                } else {
                    (
                        ResolvedImportPattern::new(vec![], vec![], vec![]),
                        vec![ParseError::ExportNotFound(*span)],
                    )
                }
            }
            ImportPatternMember::Glob { .. } => {
                let mut decls = vec![];
                let mut submodules = vec![];
                let mut variables = vec![];
                let mut errors = vec![];

                for (_, id) in &self.submodules {
                    let submodule = working_set.get_module(*id);
                    let (sub_results, sub_errors) =
                        submodule.resolve_import_pattern(working_set, *id, &[], None);
                    decls.extend(sub_results.decls);

                    submodules.extend(sub_results.modules);
                    variables.extend(sub_results.variables);
                    errors.extend(sub_errors);
                }

                decls.extend(self.decls());
                variables.extend(self.variables.clone());
                submodules.extend(self.submodules());

                (
                    ResolvedImportPattern::new(decls, submodules, variables),
                    errors,
                )
            }
            ImportPatternMember::List { names } => {
                let mut decls = vec![];
                let mut submodules = vec![];
                let mut variables = vec![];
                let mut errors = vec![];

                for (name, span) in names {
                    if name == b"main" {
                        if let Some(main_decl_id) = self.main {
                            decls.push((final_name.clone(), main_decl_id));
                        } else {
                            errors.push(ParseError::ExportNotFound(*span));
                        }
                    } else if let Some(decl_id) = self.decls.get(name) {
                        decls.push((name.clone(), *decl_id));
                    } else if let Some(var_id) = self.variables.get(name) {
                        variables.push((name.clone(), *var_id));
                    } else if let Some(submodule_id) = self.submodules.get(name) {
                        submodules.push((name.clone(), *submodule_id));
                    } else {
                        errors.push(ParseError::ExportNotFound(*span));
                    }
                }

                (
                    ResolvedImportPattern::new(decls, submodules, variables),
                    errors,
                )
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

    pub fn vars_with_head(&self, head: &[u8]) -> Vec<(Vec<u8>, VarId)> {
        self.variables
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

    pub fn submodules(&self) -> Vec<(Vec<u8>, ModuleId)> {
        self.submodules
            .iter()
            .map(|(name, id)| (name.clone(), *id))
            .collect()
    }

    pub fn decl_names(&self) -> Vec<Vec<u8>> {
        let mut result: Vec<Vec<u8>> = self.decls.keys().cloned().collect();

        if self.main.is_some() {
            result.push(self.name.clone());
        }

        result
    }
}
