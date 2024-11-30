use crate::{
    ast::ImportPatternMember, engine::StateWorkingSet, BlockId, DeclId, FileId, ModuleId,
    ParseError, Span, Value, VarId,
};

use crate::parser_path::ParserPath;
use indexmap::IndexMap;

pub struct ResolvedImportPattern {
    pub decls: Vec<(Vec<u8>, DeclId)>,
    pub modules: Vec<(Vec<u8>, ModuleId)>,
    pub constants: Vec<(Vec<u8>, Value)>,
}

impl ResolvedImportPattern {
    pub fn new(
        decls: Vec<(Vec<u8>, DeclId)>,
        modules: Vec<(Vec<u8>, ModuleId)>,
        constants: Vec<(Vec<u8>, Value)>,
    ) -> Self {
        ResolvedImportPattern {
            decls,
            modules,
            constants,
        }
    }
}

/// Collection of definitions that can be exported from a module
#[derive(Debug, Clone)]
pub struct Module {
    pub name: Vec<u8>,
    pub decls: IndexMap<Vec<u8>, DeclId>,
    pub submodules: IndexMap<Vec<u8>, ModuleId>,
    pub constants: IndexMap<Vec<u8>, VarId>,
    pub env_block: Option<BlockId>, // `export-env { ... }` block
    pub main: Option<DeclId>,       // `export def main`
    pub span: Option<Span>,
    pub imported_modules: Vec<ModuleId>, // use other_module.nu
    pub file: Option<(ParserPath, FileId)>,
}

impl Module {
    pub fn new(name: Vec<u8>) -> Self {
        Module {
            name,
            decls: IndexMap::new(),
            submodules: IndexMap::new(),
            constants: IndexMap::new(),
            env_block: None,
            main: None,
            span: None,
            imported_modules: vec![],
            file: None,
        }
    }

    pub fn from_span(name: Vec<u8>, span: Span) -> Self {
        Module {
            name,
            decls: IndexMap::new(),
            submodules: IndexMap::new(),
            constants: IndexMap::new(),
            env_block: None,
            main: None,
            span: Some(span),
            imported_modules: vec![],
            file: None,
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
        self.constants.insert(name, var_id)
    }

    pub fn add_env_block(&mut self, block_id: BlockId) {
        self.env_block = Some(block_id);
    }

    pub fn track_imported_modules(&mut self, module_id: &[ModuleId]) {
        for m in module_id {
            self.imported_modules.push(*m)
        }
    }

    pub fn has_decl(&self, name: &[u8]) -> bool {
        if name == self.name && self.main.is_some() {
            return true;
        }

        self.decls.contains_key(name)
    }

    /// Resolve `members` from given module, which is indicated by `self_id` to import.
    ///
    /// When resolving, all modules are recorded in `imported_modules`.
    pub fn resolve_import_pattern(
        &self,
        working_set: &StateWorkingSet,
        self_id: ModuleId,
        members: &[ImportPatternMember],
        name_override: Option<&[u8]>, // name under the module was stored (doesn't have to be the same as self.name)
        backup_span: Span,
        imported_modules: &mut Vec<ModuleId>,
    ) -> (ResolvedImportPattern, Vec<ParseError>) {
        imported_modules.push(self_id);
        let final_name = name_override.unwrap_or(&self.name).to_vec();

        let (head, rest) = if let Some((head, rest)) = members.split_first() {
            (head, rest)
        } else {
            // Import pattern was just name without any members
            let mut decls = vec![];
            let mut const_rows = vec![];
            let mut errors = vec![];

            for (_, id) in &self.submodules {
                let submodule = working_set.get_module(*id);
                let span = submodule.span.or(self.span).unwrap_or(backup_span);

                let (sub_results, sub_errors) = submodule.resolve_import_pattern(
                    working_set,
                    *id,
                    &[],
                    None,
                    span,
                    imported_modules,
                );
                errors.extend(sub_errors);

                for (sub_name, sub_decl_id) in sub_results.decls {
                    let mut new_name = final_name.clone();
                    new_name.push(b' ');
                    new_name.extend(sub_name);

                    decls.push((new_name, sub_decl_id));
                }

                const_rows.extend(sub_results.constants);
            }

            decls.extend(self.decls_with_head(&final_name));

            for (name, var_id) in self.consts() {
                match working_set.get_constant(var_id) {
                    Ok(const_val) => const_rows.push((name, const_val.clone())),
                    Err(err) => errors.push(err),
                }
            }

            let span = self.span.unwrap_or(backup_span);

            // only needs to bring `$module` with a record value if it defines any constants.
            let constants = if const_rows.is_empty() {
                vec![]
            } else {
                vec![(
                    normalize_module_name(&final_name),
                    Value::record(
                        const_rows
                            .into_iter()
                            .map(|(name, val)| (String::from_utf8_lossy(&name).to_string(), val))
                            .collect(),
                        span,
                    ),
                )]
            };

            return (
                ResolvedImportPattern::new(decls, vec![(final_name.clone(), self_id)], constants),
                errors,
            );
        };

        match head {
            ImportPatternMember::Name { name, span } => {
                // raise errors if user wants to do something like this:
                // `use a b c`: but b is not a sub-module of a.
                let errors = if !rest.is_empty() && self.submodules.get(name).is_none() {
                    vec![ParseError::WrongImportPattern(
                        format!("Trying to import something but the parent `{}` is not a module, maybe you want to try `use <module> [<name1>, <name2>]`", String::from_utf8_lossy(name)),
                        rest[0].span(),
                    )]
                } else {
                    vec![]
                };

                if name == b"main" {
                    if let Some(main_decl_id) = self.main {
                        (
                            ResolvedImportPattern::new(
                                vec![(final_name, main_decl_id)],
                                vec![],
                                vec![],
                            ),
                            errors,
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
                        errors,
                    )
                } else if let Some(var_id) = self.constants.get(name) {
                    match working_set.get_constant(*var_id) {
                        Ok(const_val) => (
                            ResolvedImportPattern::new(
                                vec![],
                                vec![],
                                vec![(name.clone(), const_val.clone())],
                            ),
                            errors,
                        ),
                        Err(err) => (
                            ResolvedImportPattern::new(vec![], vec![], vec![]),
                            vec![err],
                        ),
                    }
                } else if let Some(submodule_id) = self.submodules.get(name) {
                    let submodule = working_set.get_module(*submodule_id);
                    submodule.resolve_import_pattern(
                        working_set,
                        *submodule_id,
                        rest,
                        None,
                        self.span.unwrap_or(backup_span),
                        imported_modules,
                    )
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
                let mut constants = vec![];
                let mut errors = vec![];

                for (_, id) in &self.submodules {
                    let submodule = working_set.get_module(*id);
                    let (sub_results, sub_errors) = submodule.resolve_import_pattern(
                        working_set,
                        *id,
                        &[],
                        None,
                        self.span.unwrap_or(backup_span),
                        imported_modules,
                    );
                    decls.extend(sub_results.decls);

                    submodules.extend(sub_results.modules);
                    constants.extend(sub_results.constants);
                    errors.extend(sub_errors);
                }

                decls.extend(self.decls());
                constants.extend(self.constants.iter().filter_map(|(name, var_id)| {
                    match working_set.get_constant(*var_id) {
                        Ok(const_val) => Some((name.clone(), const_val.clone())),
                        Err(err) => {
                            errors.push(err);
                            None
                        }
                    }
                }));
                submodules.extend(self.submodules());

                (
                    ResolvedImportPattern::new(decls, submodules, constants),
                    errors,
                )
            }
            ImportPatternMember::List { names } => {
                let mut decls = vec![];
                let mut modules = vec![];
                let mut constants = vec![];
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
                    } else if let Some(var_id) = self.constants.get(name) {
                        match working_set.get_constant(*var_id) {
                            Ok(const_val) => constants.push((name.clone(), const_val.clone())),
                            Err(err) => errors.push(err),
                        }
                    } else if let Some(submodule_id) = self.submodules.get(name) {
                        let submodule = working_set.get_module(*submodule_id);
                        let (sub_results, sub_errors) = submodule.resolve_import_pattern(
                            working_set,
                            *submodule_id,
                            rest,
                            None,
                            self.span.unwrap_or(backup_span),
                            imported_modules,
                        );

                        decls.extend(sub_results.decls);
                        modules.extend(sub_results.modules);
                        constants.extend(sub_results.constants);
                        errors.extend(sub_errors);
                    } else {
                        errors.push(ParseError::ExportNotFound(*span));
                    }
                }

                (
                    ResolvedImportPattern::new(decls, modules, constants),
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

    pub fn consts(&self) -> Vec<(Vec<u8>, VarId)> {
        self.constants
            .iter()
            .map(|(name, id)| (name.to_vec(), *id))
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

/// normalize module names for exporting as record constant
fn normalize_module_name(bytes: &[u8]) -> Vec<u8> {
    bytes
        .iter()
        .map(|x| match is_identifier_byte(*x) {
            true => *x,
            false => b'_',
        })
        .collect()
}

fn is_identifier_byte(b: u8) -> bool {
    b != b'.'
        && b != b'['
        && b != b'('
        && b != b'{'
        && b != b'+'
        && b != b'-'
        && b != b'*'
        && b != b'^'
        && b != b'/'
        && b != b'='
        && b != b'!'
        && b != b'<'
        && b != b'>'
        && b != b'&'
        && b != b'|'
}
